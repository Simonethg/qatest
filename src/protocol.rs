use serde::{Deserialize, Serialize};

use crate::agent_state::AgentState;
use crate::tabs::TabId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Hello {
        cwd: String,
        cols: u16,
        rows: u16,
    },
    Input {
        pane_id: String,
        data_b64: String,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Focus {
        pane_id: String,
    },
    SelectTab {
        tab: TabId,
    },
    Pull {
        since: u64,
    },
    PaneSend {
        pane_id: String,
        text: String,
    },
    Detach,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    HelloOk {
        workspace: String,
        branch: String,
        protocol: u32,
    },
    Snapshot {
        seq: u64,
        workspace: String,
        branch: String,
        tab: TabId,
        focus: String,
        worst: AgentState,
        tabs: Vec<TabSnap>,
        spec_preview: String,
        review_preview: String,
        detected: Vec<String>,
    },
    Error {
        message: String,
    },
    Ack,
    Bye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSnap {
    pub id: TabId,
    pub panes: Vec<PaneSnap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneSnap {
    pub id: String,
    pub title: String,
    pub state: AgentState,
    pub occupant: String,
    /// Visible screen, one string per row (vt100 snapshot).
    pub rows: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let r = Request::SelectTab { tab: TabId::Tests };
        let s = serde_json::to_string(&r).unwrap();
        let back: Request = serde_json::from_str(&s).unwrap();
        match back {
            Request::SelectTab { tab } => assert_eq!(tab, TabId::Tests),
            _ => panic!("wrong variant"),
        }
    }
}
