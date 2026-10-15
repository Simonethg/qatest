//! TypeSafe System One client for semantic evals.
//!
//! Deterministic Playwright/pytest/axe stay the suite. These questions are
//! ISTQB CT-AI evals: typed judgments with probabilities, composed in code.
//! Docs: https://docs.typesafe.ai/api.md

use std::collections::BTreeMap;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::evals::EvalCase;

pub const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
pub const DEFAULT_MODEL: &str = "jev-latest";
const MAX_CHARS: usize = 12_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub noul: Option<f64>,
    #[serde(default)]
    pub choice: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub probabilities: BTreeMap<String, f64>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub legend: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOneResponse {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub answers: BTreeMap<String, Answer>,
    #[serde(default)]
    pub usage: Value,
}

#[derive(Debug, Clone)]
pub struct MappedEval {
    pub score: f64,
    pub detail: String,
}

pub fn api_key() -> Option<String> {
    std::env::var("TYPESAFE_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn env_disabled() -> bool {
    matches!(
        std::env::var("QATEST_TYPESAFE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "off" | "false" | "no"
    )
}

/// Unit tests never hit the network. The release binary calls System One when
/// `TYPESAFE_API_KEY` is set and `QATEST_TYPESAFE` is not off.
pub fn should_call() -> bool {
    if cfg!(test) {
        return false;
    }
    !env_disabled() && api_key().is_some()
}

pub fn skip_reason() -> String {
    if env_disabled() {
        return "SKIP TypeSafe: QATEST_TYPESAFE=0".into();
    }
    if api_key().is_none() {
        return "SKIP TypeSafe: set TYPESAFE_API_KEY (https://console.typesafe.ai)".into();
    }
    if cfg!(test) {
        return "SKIP TypeSafe: network disabled in unit tests".into();
    }
    "SKIP TypeSafe".into()
}

pub fn is_typesafe(case: &EvalCase) -> bool {
    case.kind.eq_ignore_ascii_case("typesafe")
}

pub fn model() -> String {
    std::env::var("QATEST_TYPESAFE_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

pub fn build_state(cwd: &Path, cases: &[&EvalCase]) -> Value {
    let mut want_spec = false;
    let mut want_axe = false;
    let mut want_review = false;
    for case in cases {
        let sources = state_from(case);
        if sources.is_empty() {
            want_spec = true;
            want_axe = true;
        }
        for s in sources {
            match s.as_str() {
                "spec" => want_spec = true,
                "axe" => want_axe = true,
                "review" => want_review = true,
                _ => {}
            }
        }
    }

    let mut state = serde_json::Map::new();
    state.insert(
        "policy".into(),
        json!({
            "standards": "ISTQB CT-AI, ISO/IEC/IEEE 29119, ISO/IEC 20246, WCAG 2.2 AA",
            "axe_rule": "Critical and serious axe impacts block Review. Moderate and minor are observations. Scan the changed component at 390px and 1440px; never the full page by default.",
            "evals_rule": "Deterministic file/command evals are separate. TypeSafe only judges semantic quality of the evidence pack."
        }),
    );

    if want_spec {
        state.insert("spec".into(), spec_state(cwd));
    }
    if want_axe {
        state.insert("axe".into(), axe_state(cwd));
    }
    if want_review {
        let review = crate::review::human_path(cwd);
        let human = std::fs::read_to_string(review).unwrap_or_default();
        state.insert("review_human".into(), json!(clip(&human)));
    }

    Value::Object(state)
}

fn state_from(case: &EvalCase) -> Vec<String> {
    case.state_from
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn spec_state(cwd: &Path) -> Value {
    match crate::spec::load(cwd) {
        Ok(doc) => {
            let raw = std::fs::read_to_string(&doc.path).unwrap_or_default();
            json!({
                "path": doc.path.display().to_string(),
                "status": doc.status,
                "profile": doc.profile,
                "component": doc.component,
                "acceptance": doc.acceptance.iter().map(|a| {
                    json!({
                        "id": a.id,
                        "characteristic": a.characteristic,
                        "text": a.text,
                    })
                }).collect::<Vec<_>>(),
                "raw": clip(&raw),
            })
        }
        Err(_) => json!({
            "missing": true,
            "hint": "run qatest spec init"
        }),
    }
}

fn axe_state(cwd: &Path) -> Value {
    let rows = crate::axe::load_summaries(cwd);
    if rows.is_empty() {
        return json!({
            "missing": true,
            "hint": "run qatest axe --url URL with QATEST_AXE_INCLUDE set to the changed component"
        });
    }
    json!(rows
        .iter()
        .map(|s| json!({
            "viewport": s.viewport,
            "width": s.width,
            "critical": s.critical,
            "serious": s.serious,
            "moderate": s.moderate,
            "minor": s.minor,
            "blocks_review": s.blocks_review(),
            "blockers": s.blockers,
            "observations": s.observations,
        }))
        .collect::<Vec<_>>())
}

fn clip(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= MAX_CHARS {
        return t.to_string();
    }
    t.chars().take(MAX_CHARS).collect::<String>() + "\n…[truncated]"
}

pub fn questions_payload(cases: &[&EvalCase]) -> anyhow::Result<Value> {
    let mut map = serde_json::Map::new();
    for case in cases {
        map.insert(case.id.clone(), question_object(case)?);
    }
    Ok(Value::Object(map))
}

fn question_object(case: &EvalCase) -> anyhow::Result<Value> {
    let qtype = case
        .typesafe_type
        .as_deref()
        .unwrap_or("noul")
        .to_ascii_lowercase();
    let instructions = case
        .instructions
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if instructions.is_empty() {
        bail!("{}: typesafe eval needs instructions", case.id);
    }
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!(qtype));
    obj.insert("instructions".into(), json!(instructions));
    match qtype.as_str() {
        "choice" => {
            let criteria = case.criteria.clone().unwrap_or_else(|| json!({}));
            if !criteria.is_object() {
                bail!("{}: choice criteria must be an object of option → description", case.id);
            }
            obj.insert("criteria".into(), criteria);
        }
        "score" => {
            let criteria = case.criteria.clone().unwrap_or_else(|| json!([]));
            if !criteria.is_array() {
                bail!("{}: score criteria must be an array of level descriptions", case.id);
            }
            obj.insert("criteria".into(), criteria);
        }
        "noul" => {
            if let Some(criteria) = &case.criteria {
                obj.insert("criteria".into(), criteria.clone());
            }
        }
        other => bail!("{}: unknown typesafe_type {other} (use noul, choice, or score)", case.id),
    }
    Ok(Value::Object(obj))
}

pub fn evaluate(cwd: &Path, cases: &[&EvalCase]) -> anyhow::Result<SystemOneResponse> {
    let key = api_key().context("TYPESAFE_API_KEY")?;
    let body = json!({
        "state": build_state(cwd, cases),
        "model": model(),
        "questions": questions_payload(cases)?,
    });
    let payload = serde_json::to_string(&body)?;
    let mut last = String::new();
    for attempt in 0..4 {
        match post_once(&key, &payload) {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last = format!("{e:#}");
                if attempt < 3 && retryable(&last) {
                    thread::sleep(Duration::from_millis(200 * (1 << attempt)));
                    continue;
                }
                break;
            }
        }
    }
    bail!("TypeSafe System One failed: {last}")
}

fn retryable(err: &str) -> bool {
    err.contains("429") || err.contains("529") || err.contains("timed out") || err.contains("timeout")
}

fn post_once(key: &str, payload: &str) -> anyhow::Result<SystemOneResponse> {
    let resp = ureq::post(ENDPOINT)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(45))
        .send_string(payload);
    match resp {
        Ok(r) => {
            let text = r.into_string().context("TypeSafe response body")?;
            serde_json::from_str(&text).with_context(|| format!("decode TypeSafe JSON: {text}"))
        }
        Err(ureq::Error::Status(code, r)) => {
            let text = r.into_string().unwrap_or_default();
            bail!("HTTP {code}: {text}")
        }
        Err(e) => Err(e.into()),
    }
}

pub fn map_answer(case: &EvalCase, answer: &Answer) -> MappedEval {
    let qtype = case
        .typesafe_type
        .as_deref()
        .unwrap_or("noul")
        .to_ascii_lowercase();
    match qtype.as_str() {
        "noul" => {
            let n = answer.noul.unwrap_or(0.0);
            MappedEval {
                score: n,
                detail: format!("noul={n:.3}"),
            }
        }
        "choice" => map_choice(case, answer),
        "score" => {
            let s = answer.score.unwrap_or(0.0);
            let conf = answer.confidence.unwrap_or(0.0);
            if let Some(min) = case.min_confidence {
                if conf + f64::EPSILON < min {
                    return MappedEval {
                        score: 0.0,
                        detail: format!("score={s:.3} confidence={conf:.3} below min_confidence={min:.3}"),
                    };
                }
            }
            MappedEval {
                score: s,
                detail: format!("score={s:.3} confidence={conf:.3}"),
            }
        }
        other => MappedEval {
            score: 0.0,
            detail: format!("unknown typesafe_type {other}"),
        },
    }
}

fn map_choice(case: &EvalCase, answer: &Answer) -> MappedEval {
    let choice = answer.choice.clone().unwrap_or_default();
    let conf = answer.confidence.unwrap_or(0.0);
    if let Some(min) = case.min_confidence {
        if conf + f64::EPSILON < min {
            return MappedEval {
                score: 0.0,
                detail: format!("choice={choice} confidence={conf:.3} below min_confidence={min:.3}"),
            };
        }
    }
    if let Some(expected) = case.expected.as_deref() {
        let p = answer.probabilities.get(expected).copied().unwrap_or(0.0);
        MappedEval {
            score: p,
            detail: format!("choice={choice} expected={expected} p={p:.3} confidence={conf:.3}"),
        }
    } else {
        MappedEval {
            score: 1.0,
            detail: format!("choice={choice} confidence={conf:.3}"),
        }
    }
}

pub fn answers_path(cwd: &Path) -> std::path::PathBuf {
    cwd.join(".qatest").join("typesafe-answers.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evals::EvalCase;

    fn case(kind: &str) -> EvalCase {
        EvalCase {
            id: "E-004".into(),
            name: "spec".into(),
            kind: "typesafe".into(),
            path: None,
            contains: None,
            pattern: None,
            command: None,
            traces: vec!["A-001".into()],
            threshold: 0.65,
            typesafe_type: Some(kind.into()),
            instructions: Some("Is the spec testable?".into()),
            criteria: None,
            expected: None,
            min_confidence: None,
            state_from: vec!["spec".into()],
        }
    }

    #[test]
    fn noul_maps_probability() {
        let mapped = map_answer(
            &case("noul"),
            &Answer {
                kind: "noul".into(),
                noul: Some(0.81),
                choice: None,
                score: None,
                probabilities: BTreeMap::new(),
                confidence: None,
                legend: BTreeMap::new(),
            },
        );
        assert!((mapped.score - 0.81).abs() < 1e-9);
        assert!(mapped.detail.contains("0.810"));
    }

    #[test]
    fn choice_uses_expected_probability() {
        let mut c = case("choice");
        c.expected = Some("in_component".into());
        let mut probabilities = BTreeMap::new();
        probabilities.insert("in_component".into(), 0.2);
        probabilities.insert("shared_chrome".into(), 0.8);
        let mapped = map_answer(
            &c,
            &Answer {
                kind: "choice".into(),
                noul: None,
                choice: Some("shared_chrome".into()),
                score: None,
                probabilities,
                confidence: Some(0.7),
                legend: BTreeMap::new(),
            },
        );
        assert!((mapped.score - 0.2).abs() < 1e-9);
    }

    #[test]
    fn low_confidence_fails_score() {
        let mut c = case("score");
        c.min_confidence = Some(0.6);
        let mapped = map_answer(
            &c,
            &Answer {
                kind: "score".into(),
                noul: None,
                choice: None,
                score: Some(2.0),
                probabilities: BTreeMap::new(),
                confidence: Some(0.2),
                legend: BTreeMap::new(),
            },
        );
        assert_eq!(mapped.score, 0.0);
        assert!(mapped.detail.contains("below min_confidence"));
    }
}
