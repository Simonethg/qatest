//! qat — the agent runtime for QA.

pub mod agent_state;
pub mod axe;
pub mod detect;
pub mod evals;
pub mod paths;
pub mod protocol;
pub mod pty_pane;
pub mod review;
pub mod server;
pub mod spec;
pub mod tabs;
pub mod tests_recipe;
pub mod tui;
pub mod update;
pub mod workspace;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL: u32 = 1;
pub const ENDPOINT_GENERATION: u32 = 1;
