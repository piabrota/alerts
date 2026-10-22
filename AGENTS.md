# AGENTS.md

Instructions for AI agents working on `smart-alerts`. Claude Code reads this file through
`CLAUDE.md`. OpenCode reads it directly. Both read the skills in `.claude/skills/`.

## Goal

`smart-alerts` is a generic alert engine. It collects alerts from official, authoritative
sources, filters them, and delivers them.

- **First hazard:** floods in Brazil.
- **v0 scope:** the South region: Paraná (PR), Santa Catarina (SC), Rio Grande do Sul (RS).
- **v0 sources:** Defesa Civil, INMET, CEMADEN. `README.md` lists the planned sources.
- **v0 deliverable:** a high-quality Discord service. The Discord server already exists.
- **Python interface now:** `smart-alerts-py` is a pip package from day one. The long-term
  consumer is an AstrBot plugin (for example, `astrbot_plugin_disaster_warning`).

## Hard rules

- **No CI.** Do not add `.github/workflows` or any other CI. `just check` is the gate, run by hand
  before a PR.
- **FOSS first.** A new dependency must pass `just deny` (license allowlist in `deny.toml`).
  Do not use paid resources or paid APIs without explicit human confirmation.
- **No secrets in the repo.** The Discord webhook comes from `DISCORD_WEBHOOK_URL`.
- **Official sources only.** An alert comes from an authoritative agency. Never invent an alert,
  an area, a time or an action that the source does not state.
- **No `unwrap` or `expect`** outside tests (workspace clippy lints). `unsafe` is denied.
- **`⚠` marks a claim that nobody checked.** Never remove a `⚠` without doing the check.
- **The GitHub Wiki is deployed only after the repo is public** (issue #1). Until then the pages
  live in `wiki/`.
- **Citation and licensing follow `REUSE.toml`.** Read its header before you copy, quote or cite
  anything from this repo. A new file must keep `just reuse` green. To cite the project, use
  `CITATION.cff` with a commit or a tag.

## Setup

```bash
nix develop        # the toolchain is pinned in flake.lock
just               # list recipes
just check         # fmt, clippy, tests, cargo-deny, stale translations
just test-message  # post one test alert to DISCORD_WEBHOOK_URL
just mcp           # run the MCP server on stdio
just py-build      # build the Python wheel
```

## Repo layout

A Cargo workspace. Each crate is a directory in `crates/`. Shared dependency versions and lints
live in the root `Cargo.toml` (`[workspace.dependencies]`, `[workspace.lints]`).

| Crate | Kind | Role |
|---|---|---|
| `smart-alerts-core` | lib | Domain types (`Alert`, `Uf`, `Severity`, `Hazard`) and the FP harness. Knows nothing about Discord, Python or MCP. |
| `smart-alerts` | bin | The Discord service. |
| `smart-alerts-py` | cdylib | Python bindings (PyO3 + maturin): `import smart_alerts`. |
| `smart-alerts-mcp` | bin | MCP server (rmcp, stdio) for agents. Configured in `.mcp.json` (Claude Code) and `opencode.json` (OpenCode). |

Dependencies point to `smart-alerts-core`, never the other way.

**Source facts** (checked 2026-10-22; the table and the details are in `README.en.md`):

- INMET drops the connection without a browser-like User-Agent. Always send one.
- The IDAP feed keeps about 12 hours: poll it often and store what you read.
- CPTEC/INPE forbids commercial use without authorization and requires the citation "CPTEC/INPE".
- No source blocks foreign IPs, but ANA and Epagri/Ciram registration can require a CPF or CNPJ.
- An undocumented endpoint (CEMADEN `wsAlertas2`, INMET `avisos/ativos`, ClimaRS) can change
  without notice. A module that uses one must fail loudly, never silently.

**Sources are modular.** Brazilian data is scattered, so each source is its own crate,
`crates/smart-alerts-source-<id>`, that turns the agency's data into `Alert`s. Adding a source,
or a state, adds a crate. It does not change the core.

## The FP harness

The design follows the Object-Oriented Harness paper (arXiv 2610.28380) and translates it to
functional composition. A position paper on the FP harness lives in the `smart-notes` repo.

| OO Harness | smart-alerts |
|---|---|
| A subclass overrides a method | A new `Stage` wraps the pipeline: `Harness::then` |
| `super()` | The stages before it in the composition |
| Inheritance chain | Function composition (`try_fold` over the stages) |
| Router picks a class | A pure function that matches on an enum and returns a `Harness` |
| Fall back to the base harness | `Option`/`Result` falls back to the base harness |

A `Stage` is `Fn(Alert) -> Option<Alert>`: pass the alert on, possibly changed, or drop it.
Specialize by composing a stage. Never add inheritance-like trait hierarchies.

**LLM inside, later.** Over time some sources (free-text bulletins, messy formats) need a
language model. Use an open-weight model run locally first. A paid API is an optional plug-in,
never required.

Two local runtimes, both FOSS, each reached over its local HTTP API. Each one is a `Stage`:

| Runtime | License | Job in smart-alerts | Status |
|---|---|---|---|
| Ollama | MIT | Read free text: extract fields (place, time, river level) from a bulletin into an `Alert`. | planned |
| OLLAYA (`ollaya-dev/ollaya`) | Apache-2.0 | Decide: typed questions (is this a flood alert? which severity?) with calibrated probabilities. It never generates text. | soon |

- A model never invents an alert. A model stage can only fill or drop a field that the source
  text states. When the model is unsure (below a fixed probability), the stage falls back to the
  deterministic harness and marks the alert for a human.
- The alert text that reaches Discord still follows `docs-pt-br` (`estrito`). A model does not
  write it freely.

## Why Rust

The choice is for a compiler, safety and long-term robustness (antifragility). Rust gives
algebraic types and pattern matching, one static binary without a runtime, a stable language
(editions), and PyO3 for the Python package. Scala needs the JVM and cannot ship a clean pip
package. Haskell has a thinner library ecosystem for this work.

## Documentation

Four languages, each with a controlled-language skill in `.claude/skills/`:

| Language | Skill | Standard | Written by |
|---|---|---|---|
| pt-BR | `docs-pt-br` | ptbr-writer rules | the author |
| EN | `docs-en` | ASD-STE100-style | the author |
| ZH | `docs-zh` | controlled Simplified Chinese | translated from EN |
| JA | `docs-ja` | やさしい日本語 (Easy Japanese) | translated from EN |

- `README.md` (pt-BR) and `README.en.md` mirror each other. A change to one changes the other in
  the same PR.
- Wiki pages live in `wiki/<lang>/<Page>-<lang>.md`. GitHub Wiki page names are global, so the
  language suffix keeps them unique.
- A zh or ja page starts with `<!-- source: <en page> sha: <git hash-object> reviewed: no -->`.
  `just docs-stale` lists the pages whose EN source changed.
- Alert messages are pt-BR only in v0, at the `estrito` level of `docs-pt-br`.

## Figures

A diagram is a ```` ```mermaid ```` fence in the page it illustrates, followed by a
*How to read this figure* card (*Como ler esta figura* in pt-BR): takeaway, how to read, not
shown, evidence. Where a figure and its prose disagree, the figure is the bug.
