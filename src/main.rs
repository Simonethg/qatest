use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};

use qatest::protocol::{Request, Response};
use qatest::review::HumanGate;
use qatest::{axe, detect, evals, review, server, spec, tests_recipe, tui, update, VERSION};

#[derive(Parser)]
#[command(
    name = "qatest",
    version = VERSION,
    about = "The agent runtime for QA. Powered by AcademiaQA.",
    after_help = "Not the Intel accelerator. Not the Qt tester. Source-available: use it, don't sell it."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Attach the TUI to the background server
    Attach,
    /// Background server that owns PTYs
    Server {
        #[command(subcommand)]
        action: Option<ServerCmd>,
    },
    /// Fetch latest.json (same manifest as install.sh) and replace the binary
    Update,
    /// Pane RPC (same socket the TUI uses)
    Pane {
        #[command(subcommand)]
        action: PaneCmd,
    },
    /// ISTQB / ISO 29119 spec (A# traceability)
    Spec {
        #[command(subcommand)]
        action: SpecCmd,
    },
    /// Print Playwright / pytest / axe / evals commands (does not wrap them)
    Tests {
        #[command(subcommand)]
        action: TestsCmd,
    },
    /// EvalHarness (score + threshold)
    Evals {
        #[command(subcommand)]
        action: EvalsCmd,
    },
    /// axe-core on the SUT at 390 and 1440
    Axe {
        #[arg(long)]
        url: String,
    },
    /// ISO 20246 review + ADA blockers + human-in-the-loop
    Review {
        #[command(subcommand)]
        action: ReviewCmd,
    },
    /// CLIs already on PATH
    Detect,
}

#[derive(Subcommand)]
enum ServerCmd {
    /// Run in the foreground (used by `server start`)
    Run,
    /// Daemonize
    Start,
    /// Stop the server and its panes
    Stop,
}

#[derive(Subcommand)]
enum PaneCmd {
    List,
    Send {
        id: String,
        text: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SpecCmd {
    Init,
    Status,
}

#[derive(Subcommand)]
enum TestsCmd {
    Recipe {
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Subcommand)]
enum EvalsCmd {
    Run,
}

#[derive(Subcommand)]
enum ReviewCmd {
    Status,
    Approve {
        #[arg(long)]
        note: Option<String>,
    },
    Reject {
        #[arg(long)]
        note: Option<String>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("qatest: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        None | Some(Cmd::Attach) => tui::attach(),
        Some(Cmd::Server { action }) => match action.unwrap_or(ServerCmd::Start) {
            ServerCmd::Run => server::run_foreground(),
            ServerCmd::Start => {
                server::spawn_daemon()?;
                println!("qatest server started. run `qatest` to attach.");
                Ok(())
            }
            ServerCmd::Stop => {
                server::stop()?;
                println!("qatest server stopped.");
                Ok(())
            }
        },
        Some(Cmd::Update) => update::run_update(),
        Some(Cmd::Pane { action }) => match action {
            PaneCmd::List => pane_list(),
            PaneCmd::Send { id, text } => pane_send(&id, &text.join(" ")),
        },
        Some(Cmd::Spec { action }) => match action {
            SpecCmd::Init => {
                let cwd = cwd()?;
                let path = spec::init(&cwd)?;
                println!("wrote {}", path.display());
                Ok(())
            }
            SpecCmd::Status => {
                let doc = spec::load(&cwd()?)?;
                println!("{}", spec::preview(&doc));
                Ok(())
            }
        },
        Some(Cmd::Tests { action }) => match action {
            TestsCmd::Recipe { url } => {
                print!("{}", tests_recipe::recipe(url.as_deref()));
                Ok(())
            }
        },
        Some(Cmd::Evals { action }) => match action {
            EvalsCmd::Run => {
                let report = evals::run(&cwd()?)?;
                println!("{}", evals::format_report(&report));
                if !report.all_passed() {
                    anyhow::bail!("eval harness failed");
                }
                Ok(())
            }
        },
        Some(Cmd::Axe { url }) => {
            let rows = axe::run(&cwd()?, &url)?;
            println!("{}", axe::format_summaries(&rows));
            if rows.iter().any(|r| r.blocks_review()) {
                anyhow::bail!("axe critical/serious findings block Review");
            }
            Ok(())
        },
        Some(Cmd::Review { action }) => match action {
            ReviewCmd::Status => {
                let report = review::for_cwd(&cwd()?);
                println!("{}", review::preview(&report));
                if report.verdict == review::Verdict::Blocked {
                    std::process::exit(2);
                }
                Ok(())
            }
            ReviewCmd::Approve { note } => {
                let cwd = cwd()?;
                let gate = HumanGate {
                    approved: true,
                    by: std::env::var("USER").ok(),
                    note,
                };
                review::save_human(&cwd, &gate)?;
                let report = review::for_cwd(&cwd);
                println!("{}", review::preview(&report));
                Ok(())
            }
            ReviewCmd::Reject { note } => {
                let cwd = cwd()?;
                let gate = HumanGate {
                    approved: false,
                    by: std::env::var("USER").ok(),
                    note,
                };
                review::save_human(&cwd, &gate)?;
                let report = review::for_cwd(&cwd);
                println!("{}", review::preview(&report));
                Ok(())
            }
        },
        Some(Cmd::Detect) => {
            println!("{}", detect::format_report(&cwd()?));
            Ok(())
        }
    }
}

fn cwd() -> anyhow::Result<PathBuf> {
    std::env::current_dir().context("cwd")
}

fn pane_list() -> anyhow::Result<()> {
    server::ensure_running()?;
    let size = (80u16, 24u16);
    let hello = server::rpc(Request::Hello {
        cwd: cwd()?.display().to_string(),
        cols: size.0,
        rows: size.1,
    })?;
    if let Response::Error { message } = hello {
        anyhow::bail!(message);
    }
    match server::rpc(Request::Pull { since: 0 })? {
        Response::Snapshot {
            workspace,
            branch,
            worst,
            tabs,
            ..
        } => {
            println!("{workspace}  {branch}  space={worst}");
            for tab in tabs {
                for pane in tab.panes {
                    println!(
                        "  {}  {}  {}  [{}]  {}",
                        tab.id,
                        pane.id,
                        pane.state,
                        pane.occupant,
                        pane.title
                    );
                }
            }
            Ok(())
        }
        Response::Error { message } => anyhow::bail!(message),
        _ => Ok(()),
    }
}

fn pane_send(id: &str, text: &str) -> anyhow::Result<()> {
    server::ensure_running()?;
    let _ = server::rpc(Request::Hello {
        cwd: cwd()?.display().to_string(),
        cols: 80,
        rows: 24,
    })?;
    match server::rpc(Request::PaneSend {
        pane_id: id.into(),
        text: text.into(),
    })? {
        Response::Error { message } => anyhow::bail!(message),
        _ => {
            println!("sent to {id}");
            Ok(())
        }
    }
}
