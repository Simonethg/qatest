use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Pane / space lifecycle. Worst-wins rollup: blocked > working > done > idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Blocked,
    Working,
    Done,
    Idle,
}

impl AgentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Working => "working",
            Self::Done => "done",
            Self::Idle => "idle",
        }
    }

    pub fn mark(self) -> &'static str {
        match self {
            Self::Blocked => "x",
            Self::Working => "*",
            Self::Done => "+",
            Self::Idle => "-",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Blocked => 4,
            Self::Working => 3,
            Self::Done => 2,
            Self::Idle => 1,
        }
    }

    pub fn worse(self, other: Self) -> Self {
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Infer state from a vt100 screen dump. Words first, color is not a signal.
pub fn classify(screen: &str, quiet_for: Duration, child_alive: bool) -> AgentState {
    if !child_alive {
        return AgentState::Done;
    }
    let lower = screen.to_lowercase();
    if is_blocked(&lower) {
        return AgentState::Blocked;
    }
    if quiet_for < Duration::from_millis(1500) && screen.chars().any(|c| !c.is_whitespace()) {
        return AgentState::Working;
    }
    if looks_complete(&lower) && quiet_for > Duration::from_secs(1) {
        return AgentState::Done;
    }
    AgentState::Idle
}

fn is_blocked(lower: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "waiting for",
        "awaiting",
        "(y/n)",
        "yes/no",
        "y/n)",
        "approval",
        "approve",
        "human-in-the-loop",
        "permission",
        "allow this",
        "do you want to",
        "[y/n]",
        "blocked",
    ];
    NEEDLES.iter().any(|n| lower.contains(n))
}

fn looks_complete(lower: &str) -> bool {
    lower.contains("passed")
        || lower.contains("failed")
        || lower.contains("exit_code")
        || lower.contains("tests passed")
        || lower.contains("review: blocked")
        || lower.contains("review: ready")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_beats_prompt() {
        let s = classify("Allow this tool? (y/n)", Duration::from_secs(10), true);
        assert_eq!(s, AgentState::Blocked);
    }

    #[test]
    fn dead_child_is_done() {
        assert_eq!(classify("", Duration::from_secs(0), false), AgentState::Done);
    }

    #[test]
    fn quiet_prompt_is_idle() {
        let s = classify("user@host ~ % ", Duration::from_secs(5), true);
        assert_eq!(s, AgentState::Idle);
    }

    #[test]
    fn recent_output_is_working() {
        let s = classify("running tests...\n", Duration::from_millis(200), true);
        assert_eq!(s, AgentState::Working);
    }

    #[test]
    fn rollup_prefers_blocked() {
        assert_eq!(
            AgentState::Idle.worse(AgentState::Working).worse(AgentState::Blocked),
            AgentState::Blocked
        );
    }
}
