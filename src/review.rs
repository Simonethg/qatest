use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::agent_state::AgentState;
use crate::axe::AxeSummary;
use crate::evals::EvalReport;
use crate::spec::SpecDoc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub verdict: Verdict,
    pub human: HumanGate,
    pub blockers: Vec<String>,
    pub observations: Vec<String>,
    pub spec_status: String,
    pub evals_passed: Option<bool>,
    pub axe_block: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanGate {
    pub approved: bool,
    pub by: Option<String>,
    pub note: Option<String>,
}

impl Default for HumanGate {
    fn default() -> Self {
        Self {
            approved: false,
            by: None,
            note: None,
        }
    }
}

pub fn human_path(cwd: &Path) -> std::path::PathBuf {
    cwd.join(".qat").join("review.json")
}

pub fn load_human(cwd: &Path) -> HumanGate {
    std::fs::read_to_string(human_path(cwd))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_human(cwd: &Path, gate: &HumanGate) -> anyhow::Result<()> {
    std::fs::create_dir_all(cwd.join(".qat"))?;
    std::fs::write(human_path(cwd), serde_json::to_string_pretty(gate)?)?;
    Ok(())
}

pub fn compile(
    spec: Option<&SpecDoc>,
    evals: Option<&EvalReport>,
    axe: &[AxeSummary],
    human: &HumanGate,
) -> ReviewReport {
    let mut blockers = Vec::new();
    let mut observations = Vec::new();
    let spec_status = spec
        .map(|s| s.status.clone())
        .unwrap_or_else(|| "missing".into());

    match spec {
        None => blockers.push("spec missing — run qat spec init".into()),
        Some(s) if !s.approved() => blockers.push(format!(
            "spec status is '{}' — ISO 29119 gate needs approved",
            s.status
        )),
        Some(s) if s.acceptance.is_empty() => {
            blockers.push("spec has no A# acceptance criteria".into())
        }
        Some(_) => {}
    }

    let evals_passed = evals.map(|e| e.all_passed());
    if let Some(false) = evals_passed {
        blockers.push("EvalHarness has failing cases (lesson 4.3)".into());
    }
    if evals.is_none() {
        observations.push("no eval-results.json yet — run qat evals run".into());
    }

    let mut axe_block = false;
    if axe.is_empty() {
        observations.push("no axe reports — scan the SUT at 390 and 1440".into());
    }
    for s in axe {
        if s.blocks_review() {
            axe_block = true;
            blockers.push(format!(
                "axe {} {}px: {} critical, {} serious (WCAG 2.2 AA)",
                s.viewport, s.width, s.critical, s.serious
            ));
        }
        for o in &s.observations {
            observations.push(format!("axe {}: {o}", s.viewport));
        }
    }

    if !human.approved {
        blockers.push("human-in-the-loop: qat review approve required (ISO 20246)".into());
    }

    let verdict = if blockers.is_empty() {
        Verdict::Ready
    } else {
        Verdict::Blocked
    };

    ReviewReport {
        verdict,
        human: human.clone(),
        blockers,
        observations,
        spec_status,
        evals_passed,
        axe_block,
    }
}

pub fn for_cwd(cwd: &Path) -> ReviewReport {
    let spec = crate::spec::load(cwd).ok();
    let evals = std::fs::read_to_string(crate::evals::results_path(cwd))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let axe = crate::axe::load_summaries(cwd);
    let human = load_human(cwd);
    compile(spec.as_ref(), evals.as_ref(), &axe, &human)
}

pub fn preview(report: &ReviewReport) -> String {
    let mut lines = vec![format!(
        "Review  verdict={}  spec={}  evals={:?}  axe_block={}  human={}",
        match report.verdict {
            Verdict::Ready => "ready",
            Verdict::Blocked => "blocked",
        },
        report.spec_status,
        report.evals_passed,
        report.axe_block,
        report.human.approved
    )];
    lines.push(String::new());
    if report.blockers.is_empty() {
        lines.push("no blockers".into());
    } else {
        lines.push("blockers (critical/serious / missing gates)".into());
        for b in &report.blockers {
            lines.push(format!("  BLOCK  {b}"));
        }
    }
    if !report.observations.is_empty() {
        lines.push("observations (moderate/minor — do not block)".into());
        for o in &report.observations {
            lines.push(format!("  obs    {o}"));
        }
    }
    lines.join("\n")
}

pub fn space_state(report: &ReviewReport) -> AgentState {
    match report.verdict {
        Verdict::Blocked => AgentState::Blocked,
        Verdict::Ready => AgentState::Done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::parse;
    use std::path::Path;

    #[test]
    fn draft_spec_blocks() {
        let spec = parse(Path::new("s"), "status: draft\n- A-001: thing\n");
        let report = compile(Some(&spec), None, &[], &HumanGate::default());
        assert_eq!(report.verdict, Verdict::Blocked);
        assert!(report.blockers.iter().any(|b| b.contains("approved")));
    }

    #[test]
    fn human_and_approved_spec_without_axe_can_still_block_on_human_only() {
        let spec = parse(Path::new("s"), "status: approved\n- A-001: thing\n");
        let mut human = HumanGate::default();
        human.approved = true;
        let report = compile(Some(&spec), None, &[], &human);
        assert_eq!(report.verdict, Verdict::Ready);
        assert!(report.observations.iter().any(|o| o.contains("axe")));
    }
}
