use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Instant;

use anyhow::Context;
use portable_pty::{CommandBuilder, MasterPty, PtySize};

use crate::agent_state::{self, AgentState};
use crate::tabs::TabId;

pub struct PtyPane {
    pub id: String,
    pub title: String,
    pub tab: TabId,
    pub occupant: String,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    parser: vt100::Parser,
    last_byte: Instant,
    pub seq: u64,
}

pub enum PtyEvent {
    Bytes { pane_id: String, data: Vec<u8> },
}

impl PtyPane {
    pub fn spawn(
        id: String,
        title: String,
        tab: TabId,
        occupant: String,
        cwd: &Path,
        cols: u16,
        rows: u16,
        tx: Sender<PtyEvent>,
        intro: &str,
    ) -> anyhow::Result<Self> {
        let system = portable_pty::native_pty_system();
        let pair = system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let cmd = shell_command(cwd, intro);
        let child = pair.slave.spawn_command(cmd).context("spawn pane shell")?;
        let mut reader = pair.master.try_clone_reader().context("pty reader")?;
        let writer = pair.master.take_writer().context("pty writer")?;
        let pane_id = id.clone();
        std::thread::Builder::new()
            .name(format!("qat-pty-{pane_id}"))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let _ = tx.send(PtyEvent::Bytes {
                                pane_id: pane_id.clone(),
                                data: buf[..n].to_vec(),
                            });
                        }
                        Err(_) => break,
                    }
                }
            })?;

        Ok(Self {
            id,
            title,
            tab,
            occupant,
            master: pair.master,
            writer,
            child,
            parser: vt100::Parser::new(rows, cols, 2000),
            last_byte: Instant::now(),
            seq: 0,
        })
    }

    pub fn push_bytes(&mut self, data: &[u8]) {
        self.parser.process(data);
        self.last_byte = Instant::now();
        self.seq += 1;
    }

    pub fn write_input(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.parser.set_size(rows, cols);
        Ok(())
    }

    pub fn alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    pub fn state(&mut self) -> AgentState {
        let screen = screen_text(&self.parser);
        agent_state::classify(&screen, self.last_byte.elapsed(), self.alive())
    }

    pub fn rows(&self) -> Vec<String> {
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        let mut out = Vec::with_capacity(rows as usize);
        for r in 0..rows {
            let mut line = String::new();
            for c in 0..cols {
                if let Some(cell) = screen.cell(r, c) {
                    let contents = cell.contents();
                    if contents.is_empty() {
                        line.push(' ');
                    } else {
                        line.push_str(&contents);
                    }
                }
            }
            out.push(line.trim_end().to_string());
        }
        out
    }

    pub fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }
}

fn screen_text(parser: &vt100::Parser) -> String {
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    let mut s = String::new();
    for r in 0..rows {
        for c in 0..cols {
            if let Some(cell) = screen.cell(r, c) {
                s.push_str(&cell.contents());
            }
        }
        s.push('\n');
    }
    s
}

fn shell_command(cwd: &Path, intro: &str) -> CommandBuilder {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(windows) {
            "cmd.exe".into()
        } else {
            "/bin/sh".into()
        }
    });
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-lc");
    let intro = intro.replace('\'', "");
    cmd.arg(format!(
        "printf '%s\\n' '{intro}'; cd '{}' || true; exec '{}' -l",
        cwd.display(),
        shell
    ));
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("QAT", "1");
    cmd
}
