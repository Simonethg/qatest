# Standards catalog (gates, not posters)

Every standard qat claims lives in a **tab or a gate**. If it has no gate, it is not applied.

## Canonical tabs (ISO/IEC/IEEE 29119-2 process)

| Tab | 29119-2 mapping | Gate |
| --- | --- | --- |
| Spec | Test design / conditions | Spec `status` must be `approved` before Review can pass. Acceptance IDs `A-###` must trace to at least one test or eval. |
| Code | Implementation under test | SSDF-oriented: diffs stay in the pane you already use. No extra wrapper. |
| App | Test environment | A live process pane. Env is probed, not assumed. |
| Tests | Test execution | Playwright and/or pytest for the deterministic suite. EvalHarness for evals (score ≥ threshold). axe-core on the **SUT** at 390px and 1440px. |
| Review | Test reporting / ISO 20246 | Human approval. Critical and serious axe impacts **block**. Moderate and minor are observations. |

## Layer A — core (MVP)

| Standard | Gate |
| --- | --- |
| ISTQB CTFL 4.0 | Spec template: objective, preconditions, acceptance, traceability. |
| ISTQB CT-TAE | Tests tab hosts automation you already run; qat does not replace the framework. |
| ISTQB CT-AI v1.0 / CT-GenAI 2025 | Evals sit next to the suite (not instead of it). Agents are observed (`blocked/working/done/idle`), not trusted blindly. |
| ISO/IEC/IEEE 29119-1:2022 concepts | Vocabulary in Spec (`A#`, environment, oracle). |
| ISO/IEC/IEEE 29119-2:2021 process | The five tabs **are** the process. |
| ISO/IEC/IEEE 29119-3:2021 documentation | `.qat/spec.md` and Review evidence files. |
| ISO/IEC/IEEE 29119-4:2021 techniques | Techniques named on each `A#` row when present. |
| ISO/IEC 25010:2023 | Nine quality characteristics as coverage labels in Tests (`functional suitability`, `performance efficiency`, `compatibility`, `interaction capability`, `reliability`, `security`, `maintainability`, `flexibility`, `safety`). |
| ISO/IEC 20246 | Review tab: evidence pack, verdict, human sign-off. |
| WCAG 2.2 AA | axe-core scoped to the changed SUT component. Criteria: 1.4.3, 1.4.5, 2.1.1, 2.4.7, 2.5.3, 2.5.5, 4.1.2. Viewports 390 and 1440. |

## Layer B — Americas (by SUT profile)

Applied when the spec `profile` field says so (`us`, `ca`, `latam`, `br`, `payments`).

- United States: NIST SSDF SP 800-218, NIST AI RMF, IEEE 1012-2024, Section 508 via WCAG; SOC 2 / PCI notes if `profile: payments`.
- Canada: CSA 29119, AODA, PIPEDA.
- Spanish-speaking Americas + Brazil: HASTQB; LGPD / LFPDPPP / Ley 1581 on test data.

## Layer C — post-MVP

ISO/IEC 42001, 22989, 23894, TR 29119-11, TS 42119-2, 27001, 12207, 29119-5, TR 29119-6. TMMi / CMMI as process maturity the harness *teaches*, not an audit on day one.

## Accessibility of qat itself

qat is a TUI: keyboard (prefix `Ctrl+b`, tab numbers, pane cycle) plus mouse. Agent state is a word and a mark, not color alone. Gold on ink is the accent. axe-core runs against the **product under test** in Tests, not against the binary. If a web attach ships later, scan that chrome at 390 and 1440.
