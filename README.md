# qat

**The agent runtime for QA.** Same install as an agent multiplexer. A workspace that already knows spec, tests, evals, and review.

Not the Intel QuickAssist accelerator. Not the Qt tester (`pip install qat`). This is a native binary in `~/.local/bin`.

Source-available: you may use it, copy it, and modify it. You may not sell it. See [LICENSE](LICENSE). This is not an OSI Open Source license.

Powered by AcademiaQA.

```bash
curl -fsSL https://raw.githubusercontent.com/Simonethg/qat/main/install.sh | sh
qat
```

After you buy **qat.sh** (recommended public liner — `qat.dev` is already registered by someone else), the one-liner becomes:

```bash
curl -fsSL https://qat.sh/install.sh | sh
```

Homebrew (tap, once the first release exists):

```bash
brew install Simonethg/tap/qat
```

## Why qat

Herdr is a house for agents and knows nothing about QA. Playwright agents and Testim generate or host tests and are not a `curl | sh` runtime you own. qat is the house **and** the QA process:

```text
Spec → Code → App → Tests → Review
```

Tabs are ISO/IEC/IEEE 29119-2 work, not anonymous terminals named `server`. qat does **not** wrap or replace Claude, Cursor, Playwright, or pytest. Those CLIs run in real panes, as you already run them.

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

`install.sh` is POSIX `sh`. It reads the same `latest.json` that `qat update` uses, verifies SHA-256, and installs to `${QAT_INSTALL_DIR:-$HOME/.local/bin}`. It does not edit your shell config.

Until GitHub Releases exist, `install.sh` can install a locally built binary (`QAT_LOCAL_BIN`). Development:

```bash
cargo install --path . --locked
# or
cargo build --release && cp target/release/qat ~/.local/bin/qat
```

## Commands

```bash
qat                  # start the background server if needed, attach the TUI
qat attach           # attach only
qat server start     # daemonize the server
qat server stop      # kill panes and the server
qat update           # same manifest as install.sh
qat pane list
qat pane send <id> <text>
qat spec init        # write .qat/spec.md from the ISTQB/29119 template
qat spec status
qat tests recipe     # print Playwright / pytest / axe / evals commands (does not wrap them)
qat evals run        # EvalHarness (score + threshold) from .qat/evals.json
qat axe --url URL    # axe-core at 390 and 1440, scoped when QAT_AXE_INCLUDE is set
qat review status
qat review approve   # human-in-the-loop
qat review reject
qat detect           # CLIs already on PATH (Claude, Playwright, plugins, …)
```

Detach: `Ctrl+b` then `d`. The server keeps running. `q` also detaches. Tabs: click, or `Ctrl+1`…`Ctrl+5`. Prefix `Ctrl+b` then `n` cycles panes.

## Brand

Product UI: ink / paper with SimonethG gold `#F0B000`. Footer always **Powered by AcademiaQA** (navy `#1D2047`, teal `#4CFFEE`). See [docs/brand.md](docs/brand.md).

## Standards as gates

If a standard has no gate, it is not applied. Catalog: [docs/standards.md](docs/standards.md). How agents live in the layout: [docs/ia.md](docs/ia.md).

## License

Apache License 2.0 with Commons Clause. Internal use (including by a company) is allowed. Selling qat, a fork of qat, or a hosted qat service is not.
