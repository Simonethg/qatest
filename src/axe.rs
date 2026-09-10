use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

/// Required viewports for Shopify / DTC SUT scans.
pub const VIEWPORTS: &[(u32, u32, &str)] = &[(390, 844, "mobile"), (1440, 900, "desktop")];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxeReport {
    #[serde(default)]
    pub violations: Vec<AxeViolation>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxeViolation {
    pub id: String,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub nodes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxeSummary {
    pub viewport: String,
    pub width: u32,
    pub critical: usize,
    pub serious: usize,
    pub moderate: usize,
    pub minor: usize,
    pub blockers: Vec<String>,
    pub observations: Vec<String>,
    pub path: PathBuf,
}

impl AxeSummary {
    pub fn blocks_review(&self) -> bool {
        self.critical + self.serious > 0
    }
}

pub fn summarize(report: &AxeReport, viewport: &str, width: u32, path: PathBuf) -> AxeSummary {
    let mut critical = 0;
    let mut serious = 0;
    let mut moderate = 0;
    let mut minor = 0;
    let mut blockers = Vec::new();
    let mut observations = Vec::new();
    for v in &report.violations {
        let impact = v.impact.as_deref().unwrap_or("minor").to_lowercase();
        let line = format!(
            "{} ({}) {}",
            v.id,
            impact,
            v.description.as_deref().unwrap_or("")
        );
        match impact.as_str() {
            "critical" => {
                critical += 1;
                blockers.push(line);
            }
            "serious" => {
                serious += 1;
                blockers.push(line);
            }
            "moderate" => {
                moderate += 1;
                observations.push(line);
            }
            _ => {
                minor += 1;
                observations.push(line);
            }
        }
    }
    AxeSummary {
        viewport: viewport.to_string(),
        width,
        critical,
        serious,
        moderate,
        minor,
        blockers,
        observations,
        path,
    }
}

pub fn report_path(cwd: &Path, width: u32) -> PathBuf {
    cwd.join(".qatest").join(format!("axe-{width}.json"))
}

/// Runs axe-core CLI on the SUT. Scope with QATEST_AXE_INCLUDE (CSS selector).
/// qatest does not wrap Playwright; this is the a11y gate only.
pub fn run(cwd: &Path, url: &str) -> anyhow::Result<Vec<AxeSummary>> {
    let npx = which::which("npx").context("npx not on PATH — install Node, or run the recipe in the Tests pane")?;
    std::fs::create_dir_all(cwd.join(".qatest"))?;
    let include = std::env::var("QATEST_AXE_INCLUDE").ok();
    let mut out = Vec::new();
    for (w, h, name) in VIEWPORTS {
        let dest = report_path(cwd, *w);
        let mut args = vec![
            "--yes".into(),
            "@axe-core/cli".into(),
            "--stdout".into(),
            "--tags".into(),
            "wcag22aa".into(),
        ];
        args.push(format!("--viewport-size={w},{h}"));
        if let Some(sel) = &include {
            args.push("--include".into());
            args.push(sel.clone());
        }
        args.push(url.to_string());
        let output = Command::new(&npx)
            .args(&args)
            .current_dir(cwd)
            .output()
            .context("failed to spawn axe-core")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: AxeReport = if stdout.trim().starts_with('{') {
            serde_json::from_str(stdout.trim()).unwrap_or_default()
        } else {
            AxeReport {
                url: Some(url.into()),
                violations: vec![],
            }
        };
        std::fs::write(&dest, serde_json::to_string_pretty(&parsed)?)?;
        if !output.status.success() && parsed.violations.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if parsed.violations.is_empty() && stdout.trim().is_empty() {
                bail!("axe-core failed for {w}x{h}: {stderr}");
            }
        }
        out.push(summarize(&parsed, name, *w, dest));
    }
    Ok(out)
}

pub fn load_summaries(cwd: &Path) -> Vec<AxeSummary> {
    let mut out = Vec::new();
    for (w, _, name) in VIEWPORTS {
        let path = report_path(cwd, *w);
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(report) = serde_json::from_str::<AxeReport>(&raw) {
                out.push(summarize(&report, name, *w, path));
            }
        }
    }
    out
}

pub fn format_summaries(rows: &[AxeSummary]) -> String {
    if rows.is_empty() {
        return "axe: no reports yet. run `qatest axe --url URL` (390 and 1440, scoped to the changed component).".into();
    }
    let mut lines = Vec::new();
    for s in rows {
        lines.push(format!(
            "axe {} {}px  critical={} serious={} moderate={} minor={}  {}",
            s.viewport,
            s.width,
            s.critical,
            s.serious,
            s.moderate,
            s.minor,
            if s.blocks_review() { "BLOCK" } else { "ok" }
        ));
        for b in &s.blockers {
            lines.push(format!("  BLOCK  {b}"));
        }
        for o in &s.observations {
            lines.push(format!("  obs    {o}"));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serious_blocks_review() {
        let report = AxeReport {
            url: None,
            violations: vec![AxeViolation {
                id: "color-contrast".into(),
                impact: Some("serious".into()),
                description: Some("contrast".into()),
                nodes: vec![],
            }],
        };
        let s = summarize(&report, "mobile", 390, PathBuf::from("x"));
        assert!(s.blocks_review());
        assert_eq!(s.serious, 1);
    }

    #[test]
    fn moderate_does_not_block() {
        let report = AxeReport {
            url: None,
            violations: vec![AxeViolation {
                id: "region".into(),
                impact: Some("moderate".into()),
                description: None,
                nodes: vec![],
            }],
        };
        let s = summarize(&report, "desktop", 1440, PathBuf::from("x"));
        assert!(!s.blocks_review());
    }
}
