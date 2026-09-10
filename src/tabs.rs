use serde::{Deserialize, Serialize};

/// Canonical workspace tabs. Names are the QA job, not the occupant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabId {
    Spec,
    Code,
    App,
    Tests,
    Review,
}

impl TabId {
    pub const ALL: [TabId; 5] = [
        TabId::Spec,
        TabId::Code,
        TabId::App,
        TabId::Tests,
        TabId::Review,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec => "Spec",
            Self::Code => "Code",
            Self::App => "App",
            Self::Tests => "Tests",
            Self::Review => "Review",
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Self::ALL.get(i).copied()
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % 5).unwrap()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_process_tabs() {
        assert_eq!(TabId::ALL.len(), 5);
        assert_eq!(TabId::Spec.as_str(), "Spec");
        assert_eq!(TabId::Review.index(), 4);
    }
}
