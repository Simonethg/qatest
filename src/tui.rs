use std::io;
use std::time::Duration;

use anyhow::Context;
use base64::Engine;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs as TabBar};
use ratatui::Frame;
use ratatui::Terminal;

use crate::agent_state::AgentState;
use crate::paths::Paths;
use crate::protocol::{PaneSnap, Request, Response, TabSnap};
use crate::server;
use crate::tabs::TabId;
use crate::PROTOCOL;

const INK: Color = Color::Rgb(0x0d, 0x0d, 0x0d);
const PAPER: Color = Color::Rgb(0xf5, 0xf0, 0xe8);
const GOLD: Color = Color::Rgb(0xf0, 0xb0, 0x00);
const MUTED: Color = Color::Rgb(0x8a, 0x85, 0x7a);
const BLOCKED: Color = Color::Rgb(0xe8, 0x5d, 0x4c);
const DONE: Color = Color::Rgb(0x6f, 0xbf, 0x73);
const NAVY: Color = Color::Rgb(0x1d, 0x20, 0x47);
const TEAL: Color = Color::Rgb(0x4c, 0xff, 0xee);

struct Ui {
    cwd: String,
    snapshot: Option<Response>,
    prefix: bool,
    sidebar: u16,
    status: String,
}

pub fn attach() -> anyhow::Result<()> {
    server::ensure_running()?;
    let cwd = std::env::current_dir()?;
    let mut stream = server::connect()?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let size = terminal.size()?;
    send(
        &mut stream,
        &Request::Hello {
            cwd: cwd.display().to_string(),
            cols: size.width,
            rows: size.height,
        },
    )?;
    let _ = recv(&mut stream);

    let mut ui = Ui {
        cwd: cwd.display().to_string(),
        snapshot: None,
        prefix: false,
        sidebar: 24,
        status: "Ctrl+b d detach · Ctrl+1-5 tabs · click to focus".into(),
    };

    let result = run_loop(&mut terminal, &mut stream, &mut ui);
    let _ = send(&mut stream, &Request::Detach);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    stream: &mut std::os::unix::net::UnixStream,
    ui: &mut Ui,
) -> anyhow::Result<()> {
    loop {
        send(stream, &Request::Pull { since: 0 })?;
        if let Ok(resp) = recv(stream) {
            if matches!(resp, Response::Bye) {
                break;
            }
            ui.snapshot = Some(resp);
        }

        terminal.draw(|f| draw(f, ui))?;

        if !event::poll(Duration::from_millis(33))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) => {
                if handle_key(key, stream, ui)? {
                    break;
                }
            }
            Event::Mouse(m) => handle_mouse(m, stream, ui, terminal.size()?)?,
            Event::Resize(cols, rows) => {
                send(stream, &Request::Resize { cols, rows })?;
                let _ = recv(stream);
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    stream: &mut std::os::unix::net::UnixStream,
    ui: &mut Ui,
) -> anyhow::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        ui.prefix = true;
        ui.status = "prefix: d detach  1-5 tab  n next pane  h sidebar".into();
        return Ok(false);
    }

    if ui.prefix {
        ui.prefix = false;
        match key.code {
            KeyCode::Char('d') | KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('h') => {
                ui.sidebar = if ui.sidebar > 0 { 0 } else { 24 };
            }
            KeyCode::Char('n') => cycle_pane(stream, ui)?,
            KeyCode::Char(c @ '1'..='5') => {
                let idx = (c as u8 - b'1') as usize;
                if let Some(tab) = TabId::from_index(idx) {
                    send(stream, &Request::SelectTab { tab })?;
                    ui.snapshot = recv(stream).ok();
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c @ '1'..='5') = key.code {
            let idx = (c as u8 - b'1') as usize;
            if let Some(tab) = TabId::from_index(idx) {
                send(stream, &Request::SelectTab { tab })?;
                ui.snapshot = recv(stream).ok();
            }
            return Ok(false);
        }
    }

    if let Some(id) = focused_id(ui) {
        let bytes = key_to_bytes(key);
        if !bytes.is_empty() {
            send(
                stream,
                &Request::Input {
                    pane_id: id,
                    data_b64: base64::engine::general_purpose::STANDARD.encode(&bytes),
                },
            )?;
            let _ = recv(stream);
        }
    }
    Ok(false)
}

fn handle_mouse(
    m: MouseEvent,
    stream: &mut std::os::unix::net::UnixStream,
    ui: &mut Ui,
    area: ratatui::layout::Size,
) -> anyhow::Result<()> {
    if m.kind != MouseEventKind::Down(MouseButton::Left) {
        return Ok(());
    }
    // Tab row is y=0 after sidebar split — approximate: y==0 in the main column.
    if m.column >= ui.sidebar && m.row == 0 {
        let x = m.column.saturating_sub(ui.sidebar);
        let idx = (x / 10) as usize;
        if let Some(tab) = TabId::from_index(idx.min(4)) {
            send(stream, &Request::SelectTab { tab })?;
            ui.snapshot = recv(stream).ok();
        }
        return Ok(());
    }
    if let Some(Response::Snapshot { tabs, tab, .. }) = &ui.snapshot {
        let current = tabs.iter().find(|t| t.id == *tab);
        if let Some(t) = current {
            if t.panes.len() >= 2 && m.column >= ui.sidebar {
                let mid = ui.sidebar + (area.width.saturating_sub(ui.sidebar)) / 2;
                let id = if m.column < mid {
                    t.panes[0].id.clone()
                } else {
                    t.panes[1].id.clone()
                };
                send(stream, &Request::Focus { pane_id: id })?;
                ui.snapshot = recv(stream).ok();
            }
        }
    }
    Ok(())
}

fn cycle_pane(
    stream: &mut std::os::unix::net::UnixStream,
    ui: &mut Ui,
) -> anyhow::Result<()> {
    let Some(Response::Snapshot { tabs, tab, focus, .. }) = &ui.snapshot else {
        return Ok(());
    };
    let Some(t) = tabs.iter().find(|t| t.id == *tab) else {
        return Ok(());
    };
    if t.panes.is_empty() {
        return Ok(());
    }
    let pos = t.panes.iter().position(|p| p.id == *focus).unwrap_or(0);
    let next = t.panes[(pos + 1) % t.panes.len()].id.clone();
    send(stream, &Request::Focus { pane_id: next })?;
    ui.snapshot = recv(stream).ok();
    Ok(())
}

fn focused_id(ui: &Ui) -> Option<String> {
    match &ui.snapshot {
        Some(Response::Snapshot { focus, .. }) => Some(focus.clone()),
        _ => None,
    }
}

fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            vec![(c.to_ascii_lowercase() as u8) & 0x1f]
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        _ => Vec::new(),
    }
}

fn draw(f: &mut Frame, ui: &Ui) {
    let bg = Block::default().style(Style::default().bg(INK).fg(PAPER));
    f.render_widget(bg, f.area());

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(ui.sidebar), Constraint::Min(20)])
        .split(f.area());

    if ui.sidebar > 0 {
        draw_sidebar(f, cols[0], ui);
    }
    draw_main(f, cols[1], ui);
}

fn draw_sidebar(f: &mut Frame, area: Rect, ui: &Ui) {
    let (name, branch, worst, detected, tabs) = match &ui.snapshot {
        Some(Response::Snapshot {
            workspace,
            branch,
            worst,
            detected,
            tabs,
            ..
        }) => (
            workspace.clone(),
            branch.clone(),
            *worst,
            detected.clone(),
            tabs.clone(),
        ),
        _ => (
            "qatest".into(),
            "…".into(),
            AgentState::Idle,
            Vec::new(),
            Vec::new(),
        ),
    };

    let mut lines = vec![
        Line::from(Span::styled(
            " qatest",
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(" agent runtime for QA", Style::default().fg(MUTED))),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!(" {} ", worst.mark()), state_style(worst)),
            Span::styled(name, Style::default().fg(PAPER)),
        ]),
        Line::from(Span::styled(format!("  {branch}"), Style::default().fg(MUTED))),
        Line::from(""),
        Line::from(Span::styled(" panes", Style::default().fg(GOLD))),
    ];
    for tab in &tabs {
        for pane in &tab.panes {
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", pane.state.mark()), state_style(pane.state)),
                Span::raw(format!("{} {} [{}]", pane.id, pane.state, pane.occupant)),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" detect", Style::default().fg(GOLD))));
    if detected.is_empty() {
        lines.push(Line::from(Span::styled("  (none on PATH)", Style::default().fg(MUTED))));
    }
    for d in detected.iter().take(12) {
        lines.push(Line::from(Span::styled(
            format!("  {d}"),
            Style::default().fg(PAPER),
        )));
    }

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(MUTED))
        .style(Style::default().bg(INK).fg(PAPER));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_main(f: &mut Frame, area: Rect, ui: &Ui) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let selected = match &ui.snapshot {
        Some(Response::Snapshot { tab, .. }) => tab.index(),
        _ => 0,
    };
    let titles: Vec<Line> = TabId::ALL
        .iter()
        .map(|t| Line::from(format!(" {} ", t.as_str())))
        .collect();
    let tabs = TabBar::new(titles)
        .select(selected)
        .style(Style::default().fg(MUTED).bg(INK))
        .highlight_style(
            Style::default()
                .fg(INK)
                .bg(GOLD)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    draw_panes(f, chunks[1], ui);
    draw_context(f, chunks[2], ui);

    let footer = Line::from(vec![
        Span::styled(" Powered by ", Style::default().fg(TEAL).bg(NAVY)),
        Span::styled(
            "AcademiaQA ",
            Style::default()
                .fg(TEAL)
                .bg(NAVY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  protocol {PROTOCOL}  {}", ui.cwd, ui.status),
            Style::default().fg(MUTED).bg(INK),
        ),
    ]);
    f.render_widget(Paragraph::new(footer).style(Style::default().bg(NAVY)), chunks[3]);
}

fn draw_context(f: &mut Frame, area: Rect, ui: &Ui) {
    let text = match &ui.snapshot {
        Some(Response::Snapshot {
            tab,
            spec_preview,
            review_preview,
            ..
        }) => match tab {
            TabId::Spec => spec_preview.lines().next().unwrap_or("Spec").to_string(),
            TabId::Review => review_preview.lines().next().unwrap_or("Review").to_string(),
            TabId::Tests => "Tests · suite + evals + axe 390/1440 · qatest tests recipe".into(),
            TabId::Code => "Code · run the agent you already use".into(),
            TabId::App => "App · SUT lives here · detach keeps it running".into(),
        },
        _ => "connecting…".into(),
    };
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(GOLD).bg(INK)),
        area,
    );
}

fn draw_panes(f: &mut Frame, area: Rect, ui: &Ui) {
    let Some(Response::Snapshot {
        tabs,
        tab,
        focus,
        spec_preview,
        review_preview,
        ..
    }) = &ui.snapshot
    else {
        f.render_widget(Paragraph::new("starting panes…"), area);
        return;
    };
    let Some(current) = tabs.iter().find(|t| t.id == *tab) else {
        return;
    };

    // Spec / Review: left chrome document, right PTY (or split PTYs).
    match tab {
        TabId::Spec => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(area);
            render_doc(f, split[0], " spec.md ", spec_preview, false);
            render_pty_group(f, split[1], current, focus);
        }
        TabId::Review => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(area);
            render_doc(f, split[0], " review ", review_preview, review_preview.contains("blocked"));
            render_pty_group(f, split[1], current, focus);
        }
        _ => render_pty_group(f, area, current, focus),
    }
}

fn render_doc(f: &mut Frame, area: Rect, title: &str, body: &str, blocked: bool) {
    let border = if blocked { BLOCKED } else { GOLD };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(INK).fg(PAPER));
    f.render_widget(Paragraph::new(body).block(block).scroll((0, 0)), area);
}

fn render_pty_group(f: &mut Frame, area: Rect, tab: &TabSnap, focus: &str) {
    if tab.panes.is_empty() {
        f.render_widget(Paragraph::new("no panes"), area);
        return;
    }
    if tab.panes.len() == 1 {
        render_pty(f, area, &tab.panes[0], tab.panes[0].id == focus);
        return;
    }
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    for (i, pane) in tab.panes.iter().take(2).enumerate() {
        render_pty(f, split[i], pane, pane.id == focus);
    }
}

fn render_pty(f: &mut Frame, area: Rect, pane: &PaneSnap, focused: bool) {
    let border = if focused { GOLD } else { MUTED };
    let title = format!(
        " {} {} [{}] {} ",
        pane.state.mark(),
        pane.title,
        pane.occupant,
        pane.state
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(INK).fg(PAPER));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines: Vec<Line> = pane
        .rows
        .iter()
        .map(|r| Line::from(r.clone()))
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn state_style(state: AgentState) -> Style {
    match state {
        AgentState::Blocked => Style::default().fg(BLOCKED).add_modifier(Modifier::BOLD),
        AgentState::Working => Style::default().fg(GOLD),
        AgentState::Done => Style::default().fg(DONE),
        AgentState::Idle => Style::default().fg(MUTED),
    }
}

fn send(stream: &mut std::os::unix::net::UnixStream, req: &Request) -> anyhow::Result<()> {
    use std::io::Write;
    let mut s = serde_json::to_string(req)?;
    s.push('\n');
    stream.write_all(s.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn recv(stream: &mut std::os::unix::net::UnixStream) -> anyhow::Result<Response> {
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line).context("read response")?;
    if line.is_empty() {
        anyhow::bail!("server closed");
    }
    Ok(serde_json::from_str(line.trim())?)
}

pub fn paths_hint() -> anyhow::Result<String> {
    let p = Paths::resolve()?;
    Ok(format!("socket {}  log {}", p.socket.display(), p.log.display()))
}
