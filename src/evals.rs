use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalFile {
    pub version: String,
    #[serde(default)]
    pub threshold_default: f64,
    pub evals: Vec<EvalCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub traces: Vec<String>,
    #[serde(default = "one")]
    pub threshold: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub id: String,
    pub name: String,
    pub score: f64,
    pub threshold: f64,
    pub passed: bool,
    pub traces: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<EvalResult>,
}

impl EvalReport {
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && !self.results.is_empty()
    }
}

pub fn evals_path(cwd: &Path) -> PathBuf {
    cwd.join(".qatest").join("evals.json")
}

pub fn results_path(cwd: &Path) -> PathBuf {
    cwd.join(".qatest").join("eval-results.json")
}

pub fn load_or_init(cwd: &Path) -> anyhow::Result<EvalFile> {
    crate::spec::init(cwd)?;
    let path = evals_path(cwd);
    let raw = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn run(cwd: &Path) -> anyhow::Result<EvalReport> {
    let file = load_or_init(cwd)?;
    let mut results = Vec::new();
    for case in &file.evals {
        results.push(run_case(cwd, case)?);
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let report = EvalReport {
        passed,
        failed,
        results,
    };
    std::fs::create_dir_all(cwd.join(".qatest"))?;
    std::fs::write(results_path(cwd), serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

fn run_case(cwd: &Path, case: &EvalCase) -> anyhow::Result<EvalResult> {
    let (score, detail) = match case.kind.as_str() {
        "file_exists" => {
            let p = cwd.join(case.path.as_deref().unwrap_or(""));
            if p.exists() {
                (1.0, format!("exists {}", p.display()))
            } else {
                (0.0, format!("missing {}", p.display()))
            }
        }
        "file_contains" => {
            let p = cwd.join(case.path.as_deref().unwrap_or(""));
            let needle = case.contains.as_deref().unwrap_or("");
            match std::fs::read_to_string(&p) {
                Ok(s) if s.contains(needle) => (1.0, format!("found {needle:?}")),
                Ok(_) => (0.0, format!("not found {needle:?}")),
                Err(e) => (0.0, e.to_string()),
            }
        }
        "file_matches" => {
            let p = cwd.join(case.path.as_deref().unwrap_or(""));
            let pat = case.pattern.as_deref().unwrap_or("");
            match std::fs::read_to_string(&p) {
                Ok(s) if s.contains(pat) || regex_has(&s, pat) => (1.0, format!("matched {pat}")),
                Ok(_) => (0.0, format!("no match {pat}")),
                Err(e) => (0.0, e.to_string()),
            }
        }
        "command" => {
            let cmd = case.command.as_deref().unwrap_or("");
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(cwd)
                .status();
            match status {
                Ok(s) if s.success() => (1.0, format!("exit 0: {cmd}")),
                Ok(s) => (0.0, format!("exit {}: {cmd}", s.code().unwrap_or(-1))),
                Err(e) => (0.0, e.to_string()),
            }
        }
        other => (0.0, format!("unknown kind {other}")),
    };
    let passed = score + f64::EPSILON >= case.threshold;
    Ok(EvalResult {
        id: case.id.clone(),
        name: case.name.clone(),
        score,
        threshold: case.threshold,
        passed,
        traces: case.traces.clone(),
        detail,
    })
}

fn regex_has(s: &str, pat: &str) -> bool {
    // Tiny matcher: treat pattern as a simple substring class, plus A-00[0-9] style.
    if s.contains(pat) {
        return true;
    }
    if pat.contains("[0-9]") {
        let prefix = pat.split("[0-9]").next().unwrap_or("");
        return s.lines().any(|l| l.contains(prefix) && l.chars().any(|c| c.is_ascii_digit()));
    }
    false
}

pub fn format_report(report: &EvalReport) -> String {
    let mut lines = vec![format!(
        "EvalHarness  passed={}  failed={}",
        report.passed, report.failed
    )];
    for r in &report.results {
        let mark = if r.passed { "PASS" } else { "FAIL" };
        lines.push(format!(
            "  {mark}  {}  {}  score={:.2}/{:.2}  {}  traces={}",
            r.id,
            r.name,
            r.score,
            r.threshold,
            r.detail,
            r.traces.join(",")
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn file_exists_eval_passes_on_temp_spec() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("qatest-eval-{n}"));
        std::fs::create_dir_all(dir.join(".qatest")).unwrap();
        crate::spec::init(&dir).unwrap();
        let report = run(&dir).unwrap();
        assert!(report.results.iter().any(|r| r.id == "E-001" && r.passed));
        assert!(report.results.iter().any(|r| r.id == "E-002" && !r.passed));
        let _ = std::fs::remove_dir_all(dir);
    }
}
