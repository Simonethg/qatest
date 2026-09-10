# qatest

**The agent runtime for QA.** Same install as an agent multiplexer. A workspace that already knows spec, tests, evals, and review.

Not the Intel QuickAssist accelerator. Not the Qt tester (`pip install qat`). This is a native binary named `qatest` in `~/.local/bin`.

Source-available: you may use it, copy it, and modify it. You may not sell it. See [LICENSE](LICENSE). This is not an OSI Open Source license.

Powered by AcademiaQA.

```bash
curl -fsSL https://raw.githubusercontent.com/Simonethg/qatest/main/install.sh | sh
qatest
```

After you buy **qatest.sh**, the one-liner becomes:

```bash
curl -fsSL https://qatest.sh/install.sh | sh
```

Homebrew (tap, once the first release exists):

```bash
brew install Simonethg/tap/qatest
```

## Why qatest

Herdr is a house for agents and knows nothing about QA. Playwright agents and Testim generate or host tests and are not a `curl | sh` runtime you own. qatest is the house **and** the QA process:

```text
Spec → Code → App → Tests → Review
```

Tabs are ISO/IEC/IEEE 29119-2 work, not anonymous terminals named `server`. qatest does **not** wrap or replace Claude, Cursor, Playwright, or pytest. Those CLIs run in real panes, as you already run them.

## Default workspace

| Tab | Job |
| --- | --- |
| **Spec** | ISTQB / ISO 29119 spec. Acceptance criteria tagged `A#`. No coding against a spec that is not `approved`. |
| **Code** | The programming agent you already use, plus a shell. |
| **App** | System under test / mocks. One pane, one live process. |
| **Tests** | Deterministic suite (Playwright, pytest) **and** evals in the same tab. axe-core scoped at 390px and 1440px on the SUT. |
| **Review** | ISO 20246 evidence. Critical/serious WCAG findings block approval. Human-in-the-loop. |

Sidebar states, per pane and rolled up on the space: `blocked` / `working` / `done` / `idle`. Status is never color-only.

## Install contract

`install.sh` is POSIX `sh`. It reads the same `latest.json` that `qatest update` uses, verifies SHA-256, and installs to `${QATEST_INSTALL_DIR:-$HOME/.local/bin}`. It does not edit your shell config.

Until GitHub Releases exist for a target, `install.sh` can install a locally built binary (`QATEST_LOCAL_BIN`). Development:

```bash
cargo install --path . --locked
# or
cargo build --release && cp target/release/qatest ~/.local/bin/qatest
```

## Commands

```bash
qatest                  # start the background server if needed, attach the TUI
qatest attach           # attach only
qatest server start     # daemonize the server
qatest server stop      # kill panes and the server
qatest update           # same manifest as install.sh
qatest pane list
qatest pane send <id> <text>
qatest spec init        # write .qatest/spec.md from the ISTQB/29119 template
qatest spec status
qatest tests recipe     # print Playwright / pytest / axe / evals commands (does not wrap them)
qatest evals run        # EvalHarness (score + threshold) from .qatest/evals.json
qatest axe --url URL    # axe-core at 390 and 1440, scoped when QATEST_AXE_INCLUDE is set
qatest review status
qatest review approve   # human-in-the-loop
qatest review reject
qatest detect           # CLIs already on PATH (Claude, Playwright, plugins, …)
```

Detach: `Ctrl+b` then `d`. The server keeps running. Tabs: click, or `Ctrl+1`…`Ctrl+5`. Prefix `Ctrl+b` then `n` cycles panes.

## Brand

Product UI: ink / paper with gold `#F0B000`. Footer always **Powered by AcademiaQA** (navy `#1D2047`, teal `#4CFFEE`). See [docs/brand.md](docs/brand.md).

## Standards as gates

If a standard has no gate, it is not applied. Catalog: [docs/standards.md](docs/standards.md). How agents live in the layout: [docs/ia.md](docs/ia.md).

## License

Apache License 2.0 with Commons Clause. Internal use (including by a company) is allowed. Selling qatest, a fork of qatest, or a hosted qatest service is not.
