use std::path::Path;

#[derive(Debug, Clone)]
pub struct Detected {
    pub command: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub path: String,
}

/// CLIs the user already runs. qatest hosts them; it does not wrap them.
pub const KNOWN: &[(&str, &str, &str)] = &[
    ("claude", "Claude Code", "agent"),
    ("cursor", "Cursor Agent", "agent"),
    ("codex", "Codex", "agent"),
    ("opencode", "OpenCode", "agent"),
    ("grok", "Grok CLI", "agent"),
    ("playwright", "Playwright", "suite"),
    ("pytest", "pytest", "suite"),
    ("npx", "npx", "suite"),
    ("node", "Node.js", "suite"),
    ("bun", "bun", "suite"),
    ("python3", "Python", "suite"),
    ("axe", "axe-core CLI", "a11y"),
    ("qatest-explorer", "qatest-explorer", "plugin"),
    ("qatest-builder", "qatest-builder", "plugin"),
    ("qatest-runner", "qatest-runner", "plugin"),
    ("qatest-debugger", "qatest-debugger", "plugin"),
    ("qatest-healer", "qatest-healer", "plugin"),
];

pub fn scan() -> Vec<Detected> {
    let mut out = Vec::new();
    for (cmd, label, kind) in KNOWN {
        if let Ok(path) = which::which(cmd) {
            out.push(Detected {
                command: cmd,
                label,
                kind,
                path: path.display().to_string(),
            });
        }
    }
    out
}

pub fn format_report(cwd: &Path) -> String {
    let found = scan();
    let mut lines = vec![
        format!("qatest detect  cwd={}", cwd.display()),
        "Pilar 04: these stay your CLIs. qatest is the house.".to_string(),
        String::new(),
    ];
    if found.is_empty() {
        lines.push("none of the known CLIs are on PATH.".to_string());
        lines.push("install Claude / Playwright / pytest as you already do.".to_string());
    } else {
        for d in &found {
            lines.push(format!("[{}] {}  {}  {}", d.kind, d.command, d.label, d.path));
        }
    }
    let plugins = found.iter().filter(|d| d.kind == "plugin").count();
    lines.push(String::new());
    lines.push(format!(
        "plugins present: {plugins}/5  (optional — runtime works without them)"
    ));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_plugins_and_pytest() {
        let names: Vec<_> = KNOWN.iter().map(|k| k.0).collect();
        assert!(names.contains(&"pytest"));
        assert!(names.contains(&"qatest-healer"));
        assert!(names.contains(&"claude"));
    }
}
