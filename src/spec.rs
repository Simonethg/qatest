use std::path::{Path, PathBuf};

use anyhow::Context;

pub const TEMPLATE: &str = include_str!("../templates/spec.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acceptance {
    pub id: String,
    pub characteristic: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SpecDoc {
    pub path: PathBuf,
    pub status: String,
    pub profile: String,
    pub component: String,
    pub acceptance: Vec<Acceptance>,
}

impl SpecDoc {
    pub fn approved(&self) -> bool {
        self.status.eq_ignore_ascii_case("approved")
    }

    pub fn unmapped(&self) -> Vec<&Acceptance> {
        self.acceptance.iter().filter(|a| a.text.is_empty()).collect()
    }
}

pub fn spec_path(cwd: &Path) -> PathBuf {
    cwd.join(".qatest").join("spec.md")
}

pub fn init(cwd: &Path) -> anyhow::Result<PathBuf> {
    let dir = cwd.join(".qatest");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("spec.md");
    if !path.exists() {
        std::fs::write(&path, TEMPLATE)?;
    }
    let evals = dir.join("evals.json");
    if !evals.exists() {
        std::fs::write(&evals, include_str!("../templates/evals.json"))?;
    }
    Ok(path)
}

pub fn load(cwd: &Path) -> anyhow::Result<SpecDoc> {
    let path = spec_path(cwd);
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("missing spec at {} — run qatest spec init", path.display()))?;
    Ok(parse(&path, &raw))
}

pub fn parse(path: &Path, raw: &str) -> SpecDoc {
    let status = meta(raw, "status").unwrap_or_else(|| "draft".into());
    let profile = meta(raw, "profile").unwrap_or_else(|| "us".into());
    let component = meta(raw, "component").unwrap_or_default();
    SpecDoc {
        path: path.to_path_buf(),
        status,
        profile,
        component,
        acceptance: parse_acceptance(raw),
    }
}

fn meta(raw: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    raw.lines().find_map(|line| {
        let t = line.trim();
        t.strip_prefix(&prefix)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    })
}

/// IDs look like A-001 or A#001.
pub fn parse_acceptance(raw: &str) -> Vec<Acceptance> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim().trim_start_matches('-').trim();
        let id = match take_id(t) {
            Some(id) => id,
            None => continue,
        };
        let rest = t
            .split_once(':')
            .map(|(_, r)| r.trim())
            .unwrap_or("")
            .to_string();
        let (characteristic, text) = split_characteristic(&rest);
        out.push(Acceptance {
            id,
            characteristic,
            text,
        });
    }
    out
}

fn take_id(t: &str) -> Option<String> {
    if let Some(rest) = t.strip_prefix("A-") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 3 {
            return Some(format!("A-{digits}"));
        }
    }
    if let Some(rest) = t.strip_prefix("A#") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Some(format!("A-{digits:0>3}"));
        }
    }
    None
}

fn split_characteristic(rest: &str) -> (Option<String>, String) {
    let rest = rest.trim();
    if let Some(inner) = rest.strip_prefix("[characteristic:") {
        if let Some((charact, text)) = inner.split_once(']') {
            return (
                Some(charact.trim().to_string()),
                text.trim().to_string(),
            );
        }
    }
    (None, rest.to_string())
}

pub fn preview(doc: &SpecDoc) -> String {
    let mut lines = vec![
        format!("Spec  status={}  profile={}  {}", doc.status, doc.profile, doc.path.display()),
        format!("component: {}", if doc.component.is_empty() { "(set for axe scope)" } else { &doc.component }),
        String::new(),
        "Acceptance (A# → tests/evals)".into(),
    ];
    if doc.acceptance.is_empty() {
        lines.push("  (none — add A-001: …)".into());
    }
    for a in &doc.acceptance {
        let charact = a
            .characteristic
            .as_deref()
            .unwrap_or("unlabelled");
        lines.push(format!("  {}  [{}]  {}", a.id, charact, a.text));
    }
    if !doc.approved() {
        lines.push(String::new());
        lines.push("gate: status is not approved — Review cannot pass.".into());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_ids_and_25010_labels() {
        let raw = "status: approved\n- A-001: [characteristic: security] no secrets\n- A#2: keyboard works\n";
        let doc = parse(Path::new("x"), raw);
        assert!(doc.approved());
        assert_eq!(doc.acceptance.len(), 2);
        assert_eq!(doc.acceptance[0].id, "A-001");
        assert_eq!(doc.acceptance[0].characteristic.as_deref(), Some("security"));
        assert_eq!(doc.acceptance[1].id, "A-002");
    }

    #[test]
    fn template_has_three_criteria() {
        let doc = parse(Path::new("t"), TEMPLATE);
        assert_eq!(doc.status, "draft");
        assert_eq!(doc.acceptance.len(), 3);
    }
}
