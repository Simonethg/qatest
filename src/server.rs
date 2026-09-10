use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context};
use base64::Engine;

use crate::agent_state::AgentState;
use crate::paths::Paths;
use crate::protocol::{PaneSnap, Request, Response, TabSnap};
use crate::pty_pane::{PtyEvent, PtyPane};
use crate::tabs::TabId;
use crate::{detect, review, spec, workspace, PROTOCOL};

struct Space {
    cwd: PathBuf,
    name: String,
    branch: String,
    tab: TabId,
    focus: String,
    panes: HashMap<String, PtyPane>,
    pane_order: Vec<String>,
    cols: u16,
    rows: u16,
    seq: u64,
}

struct ServerState {
    space: Option<Space>,
    detected: Vec<String>,
    tx: Sender<PtyEvent>,
}

pub fn run_foreground() -> anyhow::Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure()?;
    reclaim_stale_socket(&paths)?;

    let listener = UnixListener::bind(&paths.socket)
        .with_context(|| format!("bind {}", paths.socket.display()))?;
    std::fs::write(&paths.pid, format!("{}\n", std::process::id()))?;

    let (tx, rx) = mpsc::channel::<PtyEvent>();
    let state = Arc::new(Mutex::new(ServerState {
        space: None,
        detected: detect::scan()
            .into_iter()
            .map(|d| format!("{}:{}", d.kind, d.command))
            .collect(),
        tx,
    }));

    let drain_state = state.clone();
    std::thread::spawn(move || drain_pty(rx, drain_state));

    eprintln!("qatest server listening on {}", paths.socket.display());
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let state = state.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, state) {
                        eprintln!("client: {e:#}");
                    }
                });
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
    Ok(())
}

pub fn spawn_daemon() -> anyhow::Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure()?;
    if server_alive(&paths) {
        return Ok(());
    }
    reclaim_stale_socket(&paths)?;

    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log)?;
    let err = log.try_clone()?;
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("server").arg("run");
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::from(log));
    cmd.stderr(std::process::Stdio::from(err));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn().context("spawn qatest server")?;

    for _ in 0..50 {
        if paths.socket.exists() && server_alive(&paths) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    bail!(
        "server did not come up — see {}",
        paths.log.display()
    );
}

pub fn stop() -> anyhow::Result<()> {
    let paths = Paths::resolve()?;
    if let Ok(mut stream) = UnixStream::connect(&paths.socket) {
        let line = serde_json::to_string(&Request::Stop)? + "\n";
        let _ = stream.write_all(line.as_bytes());
        let _ = stream.flush();
        std::thread::sleep(Duration::from_millis(80));
    }
    if let Ok(pid_s) = std::fs::read_to_string(&paths.pid) {
        if let Ok(pid) = pid_s.trim().parse::<i32>() {
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
        }
    }
    let _ = std::fs::remove_file(&paths.socket);
    let _ = std::fs::remove_file(&paths.pid);
    Ok(())
}

pub fn ensure_running() -> anyhow::Result<()> {
    spawn_daemon()
}

pub fn connect() -> anyhow::Result<UnixStream> {
    let paths = Paths::resolve()?;
    UnixStream::connect(&paths.socket)
        .with_context(|| format!("connect {} — is the server running?", paths.socket.display()))
}

fn server_alive(paths: &Paths) -> bool {
    UnixStream::connect(&paths.socket).is_ok()
}

fn reclaim_stale_socket(paths: &Paths) -> anyhow::Result<()> {
    if paths.socket.exists() && UnixStream::connect(&paths.socket).is_err() {
        let _ = std::fs::remove_file(&paths.socket);
        let _ = std::fs::remove_file(&paths.pid);
    }
    Ok(())
}

fn drain_pty(rx: Receiver<PtyEvent>, state: Arc<Mutex<ServerState>>) {
    while let Ok(ev) = rx.recv() {
        match ev {
            PtyEvent::Bytes { pane_id, data } => {
                let mut st = state.lock().expect("server mutex");
                if let Some(space) = st.space.as_mut() {
                    if let Some(pane) = space.panes.get_mut(&pane_id) {
                        pane.push_bytes(&data);
                        space.seq += 1;
                    }
                }
            }
        }
    }
}

fn handle_client(stream: UnixStream, state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    stream.set_read_timeout(None)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let req: Request = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                write_resp(&mut writer, &Response::Error { message: e.to_string() })?;
                continue;
            }
        };
        match req {
            Request::Stop => {
                write_resp(&mut writer, &Response::Bye)?;
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(30));
                    std::process::exit(0);
                });
                break;
            }
            Request::Detach => {
                write_resp(&mut writer, &Response::Bye)?;
                break;
            }
            other => {
                let resp = dispatch(other, &state);
                write_resp(&mut writer, &resp)?;
            }
        }
    }
    Ok(())
}

fn write_resp(w: &mut UnixStream, resp: &Response) -> anyhow::Result<()> {
    let mut s = serde_json::to_string(resp)?;
    s.push('\n');
    w.write_all(s.as_bytes())?;
    w.flush()?;
    Ok(())
}

fn dispatch(req: Request, state: &Arc<Mutex<ServerState>>) -> Response {
    match req {
        Request::Hello { cwd, cols, rows } => {
            let mut st = state.lock().expect("server mutex");
            if let Err(e) = ensure_space(&mut st, PathBuf::from(cwd), cols, rows) {
                return Response::Error {
                    message: format!("{e:#}"),
                };
            }
            let space = st.space.as_ref().unwrap();
            Response::HelloOk {
                workspace: space.name.clone(),
                branch: space.branch.clone(),
                protocol: PROTOCOL,
            }
        }
        Request::Pull { .. } => snapshot(state),
        Request::Input { pane_id, data_b64 } => {
            match base64::engine::general_purpose::STANDARD.decode(data_b64) {
                Ok(bytes) => {
                    let mut st = state.lock().expect("server mutex");
                    if let Some(space) = st.space.as_mut() {
                        if let Some(pane) = space.panes.get_mut(&pane_id) {
                            if let Err(e) = pane.write_input(&bytes) {
                                return Response::Error {
                                    message: e.to_string(),
                                };
                            }
                        }
                    }
                    Response::Ack
                }
                Err(e) => Response::Error {
                    message: e.to_string(),
                },
            }
        }
        Request::PaneSend { pane_id, text } => {
            let mut st = state.lock().expect("server mutex");
            if let Some(space) = st.space.as_mut() {
                if let Some(pane) = space.panes.get_mut(&pane_id) {
                    let mut data = text.into_bytes();
                    if !data.ends_with(&[b'\n']) {
                        data.push(b'\n');
                    }
                    if let Err(e) = pane.write_input(&data) {
                        return Response::Error {
                            message: e.to_string(),
                        };
                    }
                }
            }
            snapshot_locked(&mut st)
        }
        Request::Focus { pane_id } => {
            let mut st = state.lock().expect("server mutex");
            if let Some(space) = st.space.as_mut() {
                if space.panes.contains_key(&pane_id) {
                    space.focus = pane_id;
                }
            }
            snapshot_locked(&mut st)
        }
        Request::SelectTab { tab } => {
            let mut st = state.lock().expect("server mutex");
            if let Some(space) = st.space.as_mut() {
                space.tab = tab;
                if let Some(id) = space
                    .pane_order
                    .iter()
                    .find(|id| space.panes.get(*id).map(|p| p.tab) == Some(tab))
                {
                    space.focus = id.clone();
                }
            }
            snapshot_locked(&mut st)
        }
        Request::Resize { cols, rows } => {
            let mut st = state.lock().expect("server mutex");
            if let Some(space) = st.space.as_mut() {
                space.cols = cols.max(20);
                space.rows = rows.max(8);
                let (c, r) = pane_size(space.cols, space.rows);
                for pane in space.panes.values_mut() {
                    let _ = pane.resize(c, r);
                }
            }
            snapshot_locked(&mut st)
        }
        Request::Detach | Request::Stop => snapshot(state),
    }
}

fn snapshot(state: &Arc<Mutex<ServerState>>) -> Response {
    let mut st = state.lock().expect("server mutex");
    snapshot_locked(&mut st)
}

fn snapshot_locked(st: &mut ServerState) -> Response {
    let detected = st.detected.clone();
    let Some(space) = st.space.as_mut() else {
        return Response::Error {
            message: "no workspace — send hello first".into(),
        };
    };
    let spec_preview = spec::load(&space.cwd)
        .map(|d| spec::preview(&d))
        .unwrap_or_else(|e| format!("spec: {e:#}"));
    let review_preview = review::preview(&review::for_cwd(&space.cwd));

    let mut tabs = Vec::new();
    let mut worst = AgentState::Idle;
    for tab in TabId::ALL {
        let mut panes = Vec::new();
        for id in &space.pane_order {
            let pane = match space.panes.get_mut(id) {
                Some(p) if p.tab == tab => p,
                _ => continue,
            };
            let state = pane.state();
            worst = worst.worse(state);
            let (cr, cc) = pane.cursor();
            panes.push(PaneSnap {
                id: pane.id.clone(),
                title: pane.title.clone(),
                state,
                occupant: pane.occupant.clone(),
                rows: pane.rows(),
                cursor_row: cr,
                cursor_col: cc,
            });
        }
        tabs.push(TabSnap { id: tab, panes });
    }

    Response::Snapshot {
        seq: space.seq,
        workspace: space.name.clone(),
        branch: space.branch.clone(),
        tab: space.tab,
        focus: space.focus.clone(),
        worst,
        tabs,
        spec_preview,
        review_preview,
        detected,
    }
}

fn ensure_space(
    st: &mut ServerState,
    cwd: PathBuf,
    cols: u16,
    rows: u16,
) -> anyhow::Result<()> {
    if let Some(space) = st.space.as_ref() {
        if space.cwd == cwd {
            return Ok(());
        }
    }
    let _ = spec::init(&cwd);
    let (name, branch) = workspace::git_name_and_branch(&cwd);
    let cols = cols.max(40);
    let rows = rows.max(12);
    let (pc, pr) = pane_size(cols, rows);
    let mut space = Space {
        cwd: cwd.clone(),
        name,
        branch,
        tab: TabId::Spec,
        focus: String::new(),
        panes: HashMap::new(),
        pane_order: Vec::new(),
        cols,
        rows,
        seq: 0,
    };

    let seeds = [
        (
            "spec-doc",
            TabId::Spec,
            "spec",
            "cat .qatest/spec.md; echo; echo '---'; echo 'edit .qatest/spec.md  |  qatest spec status'",
        ),
        (
            "spec-shell",
            TabId::Spec,
            "shell",
            "echo Spec shell. A# IDs must trace to tests.",
        ),
        (
            "code-agent",
            TabId::Code,
            "agent",
            "echo Code pane. Run claude / cursor / codex here. qatest does not wrap them.",
        ),
        (
            "app-sut",
            TabId::App,
            "sut",
            "echo App pane. Start the SUT or mocks here. Detach will not kill this process.",
        ),
        (
            "tests-suite",
            TabId::Tests,
            "suite",
            "echo Tests suite. npx playwright test   or   pytest -q",
        ),
        (
            "tests-evals",
            TabId::Tests,
            "evals",
            "echo Evals + axe. qatest evals run   |   qatest axe --url URL",
        ),
        (
            "review-gate",
            TabId::Review,
            "review",
            "echo Review. qatest review status   |   qatest review approve",
        ),
        (
            "review-evidence",
            TabId::Review,
            "evidence",
            "echo Evidence pane. cat .qatest/eval-results.json .qatest/axe-390.json .qatest/axe-1440.json",
        ),
    ];

    for (id, tab, occupant, intro) in seeds {
        let pane = PtyPane::spawn(
            id.into(),
            id.into(),
            tab,
            occupant.into(),
            &cwd,
            pc,
            pr,
            st.tx.clone(),
            intro,
        )?;
        space.pane_order.push(pane.id.clone());
        space.panes.insert(pane.id.clone(), pane);
    }
    space.focus = "spec-doc".into();
    st.space = Some(space);
    Ok(())
}

fn pane_size(cols: u16, rows: u16) -> (u16, u16) {
    // sidebar ~22, tab bar 1, footer 1, split two panes.
    let inner_cols = cols.saturating_sub(24).max(20) / 2;
    let inner_rows = rows.saturating_sub(4).max(8);
    (inner_cols, inner_rows)
}

pub fn rpc(req: Request) -> anyhow::Result<Response> {
    let mut stream = connect()?;
    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    stream.write_all(line.as_bytes())?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut resp = String::new();
    reader.read_line(&mut resp)?;
    Ok(serde_json::from_str(resp.trim())?)
}
