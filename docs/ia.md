# Agents in the workspace

qatest is not the agent. qatest is the house. Pilar 04: we do not wrap or replace Claude, Cursor, Codex, Playwright, or pytest.

## Where they live

| Tab | Typical occupant | qatest does |
| --- | --- | --- |
| Spec | Editor / spec agent | Parses `A#`, refuses Review if status ≠ `approved`. |
| Code | Claude Code, Cursor Agent, Codex, OpenCode | Detects the CLI on PATH, hosts it in a PTY, reports `blocked/working/done/idle`. |
| App | SUT, mocks, `npm run dev` | Keeps the process alive after detach. |
| Tests | `npx playwright test`, `pytest`, EvalHarness, axe | Hosts them. `qatest tests recipe` prints the commands. `qatest evals run` and `qatest axe` are thin runners, not a new framework. |
| Review | Human | Blocks on critical/serious a11y and missing approval. |

## Detection

`qatest detect` lists CLIs already installed. Known names:

- Agents: `claude`, `cursor`, `codex`, `opencode`, `grok`
- Suite: `playwright`, `pytest`, `npx`, `node`, `bun`
- Plugins (optional, not the product): `qatest-explorer`, `qatest-builder`, `qatest-runner`, `qatest-debugger`, `qatest-healer`

If a plugin is missing, the runtime still works: Claude on the left, `npx playwright test` on the right.

## Lifecycle events (socket API)

The server owns PTYs. The client is a viewer. Events are JSON over the local socket (`qatest pane send` is the same surface):

| Event | Meaning |
| --- | --- |
| `thought` / output bytes | Pane PTY data. Not a fake “thinking” panel. |
| `tool` | Inferred from screen text (shell commands, Playwright, pytest). |
| `blocked` | Pane is waiting (approval, y/n, human). |
| `working` | Recent output, child alive. |
| `done` | Child exited or suite reported completion. |
| `idle` | Prompt-like, quiet. |

There is no cloud WebSocket in the MVP. Detach does not kill occupants. `qatest server stop` does.

## Evals vs suite (lesson 3.1 / 4.3)

The suite is deterministic (Playwright, pytest). Evals score an agent or model against a threshold (`.qatest/evals.json`). Both run in Tests. Review reads both result files. Changing a system prompt without evals is out of process.
