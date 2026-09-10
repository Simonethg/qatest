# Spec

status: draft
standard: ISO/IEC/IEEE 29119-2:2021, ISO/IEC/IEEE 29119-3:2021, ISTQB CTFL 4.0
profile: us
product: (system under test)
component: (changed component — axe scope)

## Objective

What must be true for this change to be done.

## Preconditions

Probed, not assumed. Environment lives in the App tab.

## Acceptance criteria

- A-001: [characteristic: functional suitability] Describe the observable result. Technique: (29119-4).
- A-002: [characteristic: interaction capability] Keyboard and name/role/value still work on the changed component.
- A-003: [characteristic: security] No new secret in client logs.

## Traceability

| ID | Tests | Evals | Status |
| --- | --- | --- | --- |
| A-001 | | | unmapped |
| A-002 | | | unmapped |
| A-003 | | | unmapped |

## Notes

`status` must be `approved` before Review can pass. Each `A-###` must map to a test or an eval. qat does not generate tests unless you install `qat-builder` (plugin, optional).
