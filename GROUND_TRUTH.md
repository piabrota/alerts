# GROUND_TRUTH.md

Instructions for agents working on `alerts`. The skills live in `verbs/`: one directory per
verb, each with a `SKILL.md`. `scripts/link-verbs.sh` makes a local agent read this file and
`verbs/`, with links that git ignores. `just pack-verbs` writes one zip per verb in
`target/verbs/`, to upload as a project skill. OpenCode reads this file through `instructions` in
`opencode.json`. `AGENTS.md` is in `.gitignore`: never commit a file with that name. No file or
directory name in this repo names an AI product.

## Goal

`alerts` is a generic alert engine. It collects alerts from official, authoritative
sources, filters them, and delivers them.

- **First hazard:** floods in Brazil.
- **v0 scope:** the South region: Paraná (PR), Santa Catarina (SC), Rio Grande do Sul (RS).
- **v0 sources:** Defesa Civil, INMET, CEMADEN. `README.md` lists the planned sources.
- **v0 deliverable:** a high-quality Discord service. The Discord server already exists.
- **Second channel, with parity:** an Atom feed (RFC 4287) whose entries hold CAP 1.2 alerts
  (OASIS). Open standards, no company attached. Every alert that reaches Discord also reaches the
  feed, with the same pt-BR text. A new channel reads the same harness output: never filter or
  rewrite alerts per channel.
- **Python interface now:** `alerts-py` is a pip package from day one. The long-term
  consumer is an AstrBot plugin (for example, `astrbot_plugin_disaster_warning`).
- **Codeberg mirror without AI trace:** `codeberg.org/piabrota/alerts` holds the code with no
  trace of AI tools. No agent file (`AGENTS.md`, `verbs/` as skills, `scripts/`, `opencode.json`,
  `.mcp.json`), no AI co-author trailer, and no AI tool name in a commit message. Build the copy
  only with `just export-codeberg <out-dir>` (Rust, `scripts/export-codeberg/`): it fails when a
  trace remains.
  Commit the result with a plain message as the author. Never push this repo's history there.

## Hard rules

- **Full Rust.** Every program, script and tool written for this repo is Rust. Python enters
  only as a third-party dependency: maturin builds the pip package, `uvx reuse` checks the
  licenses, and the pip package itself is Rust (PyO3). Never commit a `.py` file. A shell recipe
  in the `justfile` may call tools, but a recipe that needs logic becomes a Rust binary. One
  exception: `crates/alerts-obsidian/plugin/main.js`, because Obsidian loads only JavaScript. It
  is only the bridge to the Obsidian API (HTTP, files, notices, settings). The plugin logic is the
  Rust of `alerts-obsidian`, built to WebAssembly. Never add logic to that file.
- **No CI.** Do not add `.github/workflows` or any other CI. `just check` is the gate, run by hand
  before a PR.
- **Every PR delivers a test alert to the official Marola Discord.** `just marola-discord` (in
  `just check`) posts one test alert (CAP status Test, headline `TESTE: ...`) to the webhook in
  `MAROLA_DISCORD_WEBHOOK_URL`. It fails when the variable is missing, when the URL is not a Discord
  webhook, or when Discord does not store the message. Write the result in **Testado**. Never
  print the webhook URL: it holds the token.
- **Every PR uses the `open-pr` verb.** The title is pt-BR. The body is bilingual, pt-BR first, at
  the `estrito` level of `write-pt-br`, and ends with `verbs/open-pr/footer.md`. One PR per request:
  new work for an open PR goes into that PR.
- **A PR that changes an image uses the `open-pr-images` verb** (bipri): the `open-pr` body plus
  before and after images, committed in `docs/screenshots/<topic>/`.
- **Identify honestly.** Every HTTP request sends `alerts_core::USER_AGENT`
  (`piabrota-alerts/<version> (+repo URL)`). Never send a browser User-Agent and never copy another
  client's name, not even as a fallback. When a source refuses the honest name, stop and tell a
  human: the fix is to ask the agency, not to disguise the client.
- **FOSS first.** A new dependency must pass `just deny` (license allowlist in `deny.toml`).
  Do not use paid resources or paid APIs without explicit human confirmation.
- **No secrets in the repo.** The Discord webhooks come from `DISCORD_WEBHOOK_URL` (the alerts
  server) and `MAROLA_DISCORD_WEBHOOK_URL` (the official Marola server).
- **Official sources only.** An alert comes from an authoritative agency. Never invent an alert,
  an area, a time or an action that the source does not state.
- **No `unwrap` or `expect`** outside tests (workspace clippy lints). `unsafe` is denied.
- **`⚠` marks a claim that nobody checked.** Never remove a `⚠` without doing the check.
- **The GitHub Wiki is deployed only after the repo is public** (issue #1). Until then the pages
  live in `wiki/`.
- **Citation and licensing follow `REUSE.toml`.** Read its header before you copy, quote or cite
  anything from this repo. A new file must keep `just reuse` green. To cite the project, use
  `CITATION.cff` with a commit or a tag.
- **Every file for humans states its reading time as metadata.** A file for humans is a page a
  person reads: a README, a wiki page, a doc in `docs/`. The metadata is the first line of the
  file, as an HTML comment: `<!-- reading-time: <N> min -->`. In a zh or ja page, this line comes
  directly after the `source` line, and N is the N of its English source. `just reading-time`
  writes the line with the calculator of ptbr-writer (`ptbr reading-time`), and
  `just reading-time-check` (in `just check`) fails on a file without it. Code, data, licenses,
  agent files and the alert messages are not files for humans.

## Setup

```bash
nix develop        # the toolchain is pinned in flake.lock
just               # list recipes
just check         # fmt, clippy, tests, cargo-deny, stale translations
just test-message  # publish one test alert to DISCORD_WEBHOOK_URL and/or FEED_PATH
just marola-discord # the hard-rule test: one test alert to MAROLA_DISCORD_WEBHOOK_URL
just mock-alerts    # deliver the mock alerts (TESTE) to DISCORD_WEBHOOK_URL and/or FEED_PATH
just obsidian      # build the Obsidian plugin into target/obsidian/piabrota-alerts
just mcp           # run the MCP server on stdio
just py-build      # build the Python wheel
```

## Repo layout

A Cargo workspace. Each crate is a directory in `crates/`. Shared dependency versions and lints
live in the root `Cargo.toml` (`[workspace.dependencies]`, `[workspace.lints]`).

| Crate | Kind | Role |
|---|---|---|
| `alerts-core` | lib | Domain types (`Alert`, `Uf`, `Severity`, `Hazard`) and the FP harness. Knows nothing about Discord, Python or MCP. |
| `alerts` | bin + lib | The service: publishes to Discord and to the feed. `tests/marola_discord.rs` is the Marola Discord test. |
| `alerts-obsidian` | cdylib (wasm32) | WeatherDisaster, the Obsidian plugin (id `piabrota-alerts`): an Atom/RSS feed of CAP alerts becomes one note per alert. Read `docs/OBSIDIAN.md`. |
| `alerts-feed` | lib | The Atom + CAP 1.2 channel. The CAP output must validate against the OASIS CAP 1.2 XSD. A test alert is always `<status>Test</status>`. |
| `alerts-py` | cdylib | Python bindings (PyO3 + maturin): `import piabrota_alerts`. |
| `alerts-mcp` | bin | MCP server (rmcp, stdio) for agents. Configured in `.mcp.json` and `opencode.json`. |
| `alerts-drawings` | bin | `wave`: the README drawing as SVG and as a Desmos state (`just wave`). |
| `alerts-desmos` | bin | `desmos-gif`: records a Desmos animation as a GIF (`just desmos-gif`). |
| `alerts-crud` | lib | Loco-style models, controllers and migrations (SQLite) of the desktop app. `controllers::dispatch` is the one entry point. |
| `alerts-rss` | lib | The RSS combinator: RSS and Atom items, a rule algebra (union, intersect, diff, filter, map, dedupe, take) and a pure evaluator. No I/O. See `docs/RSS.md`. |
| `alerts-scaffold` | bin | `scaffold`: writes a new `alerts-crud` resource (`just generate scaffold`). |
| `alerts-source-mock` | lib | The mock source: simulated test alerts (headline `TESTE:`, CAP status Test) for `alerts poll mock` and `just mock-alerts`. Never an agency. |

`desktop/` is its own Cargo workspace, outside `crates/`: `desktop/app` (Tauri 2) and `desktop/ui` (Leptos,
wasm32). It needs WebKitGTK on Linux, and `just desktop-check` (in `just check`) is its gate. Read
`docs/DESKTOP.md` before you work on it.

Dependencies point to `alerts-core`, never the other way.

**Source facts** (checked 2026-10-22; endpoints and formats in `docs/SOURCES.md`):

- INMET drops the connection when the User-Agent is curl's default. The honest
  User-Agent worked with the former project name; nobody checked the new name ⚠ (see the hard
  rule on honest identification).
- The IDAP feed keeps about 12 hours: poll it often and store what you read.
- CPTEC/INPE forbids commercial use without authorization and requires the citation "CPTEC/INPE".
- No source blocks foreign IPs, but ANA and Epagri/Ciram registration can require a CPF or CNPJ.
- An undocumented endpoint (CEMADEN `wsAlertas2`, INMET `avisos/ativos`, ClimaRS) can change
  without notice. A module that uses one must fail loudly, never silently.

**Sources are modular.** Brazilian data is scattered, so each source is its own crate,
`crates/alerts-source-<id>`, that turns the agency's data into `Alert`s. Adding a source,
or a state, adds a crate. It does not change the core.

## The FP harness

The design follows the Object-Oriented Harness paper (arXiv 2610.28380) and translates it to
functional composition. A position paper on the FP harness lives in the `smart-notes` repo.

| OO Harness | alerts |
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

| Runtime | License | Job in alerts | Status |
|---|---|---|---|
| Ollama | MIT | Read free text: extract fields (place, time, river level) from a bulletin into an `Alert`. | planned |
| OLLAYA (`ollaya-dev/ollaya`) | Apache-2.0 | Decide: typed questions (is this a flood alert? which severity?) with calibrated probabilities. It never generates text. | soon |

- A model never invents an alert. A model stage can only fill or drop a field that the source
  text states. When the model is unsure (below a fixed probability), the stage falls back to the
  deterministic harness and marks the alert for a human.
- The alert text that reaches Discord still follows `write-pt-br` (`estrito`). A model does not
  write it freely.
- Before adding any NLP tool or model, read `docs/SURVEY_NLP.md`: use the cheapest tier that
  does the job, and record real alerts as eval cases first.

## Apps and maps

An app is one deployment: sources, a harness and channels for one audience. The conventions follow
[Loco](https://loco.rs) (generators, doctor, scheduler, tasks), without the ORM (`alerts-crud` writes
plain SQL), the accounts and the server-side HTML. Read the skills before you work on these parts:

| Skill | Job |
|---|---|
| `manage-apps` | What an app is, where its parts go, the planned `just` recipes |
| `generate` | New app, source crate, stage or channel |
| `diagnose` | Check that an app can run |
| `schedule` | Poll rates, delivery once per id, one-off tasks |
| `map` | Alert areas on a map, CAP 1.2 to GeoJSON, FOSS tiles only |

The desktop app (`docs/DESKTOP.md`) is the first interface: a CRUD page per `alerts-crud` resource and
a map page. The interface never creates an alert.

## Why Rust

The choice is for a compiler, safety and long-term robustness (antifragility). Rust gives
algebraic types and pattern matching, one static binary without a runtime, a stable language
(editions), and PyO3 for the Python package. Scala needs the JVM and cannot ship a clean pip
package. Haskell has a thinner library ecosystem for this work.

## Documentation

Four languages, each with a controlled-language skill in `verbs/`:

| Language | Skill | Standard | Written by |
|---|---|---|---|
| pt-BR | `write-pt-br` | ptbr-writer rules | the author |
| EN | `write-en` | ASD-STE100-style | the author |
| ZH | `translate-zh` | controlled Simplified Chinese | translated from EN |
| JA | `translate-ja` | やさしい日本語 (Easy Japanese) | translated from EN |

- `README.md` (EN, the landing page) and `README.pt-BR.md` mirror each other. A change to one changes the other in
  the same PR.
- Wiki pages live in `wiki/<lang>/<Page>-<lang>.md`. GitHub Wiki page names are global, so the
  language suffix keeps them unique.
- A zh or ja page starts with `<!-- source: <en page> sha: <git hash-object> reviewed: no -->`.
  `just docs-stale` lists the pages whose EN source changed.
- Alert messages are pt-BR only in v0, at the `estrito` level of `write-pt-br`.

## Figures

A diagram is a ```` ```mermaid ```` fence in the page it illustrates, or a PNG drawn with Archify
(`just figure`, the `figure` verb), followed by a *How to read this figure* card (*Como ler esta figura* in pt-BR): takeaway, how to read, not
shown, evidence. Where a figure and its prose disagree, the figure is the bug.

## Tools from ptbr-writer

These tools come from `h0ffmann/ptbr-writer` and from picasso's labs, at the revision that
ptbr-writer pins (`5da5237`). `just pins` fails when the three flake inputs differ.

| Tool | Recipe | What it does |
|---|---|---|
| ptbr-tools (`ptbr reading-time`) | `just reading-time` | Writes the reading time of every file for humans |
| Archify (`vendor/archify`, MIT, pinned `2ab3cae`) | `just figure-init`, `just figure`, `just figure-check` | Draws `docs/archify/<slug>.json` to `docs/img/<slug>.png`, with picasso's look |
| code-graph (graphify's schema) | `just graph` | The code graph of this repo: `build/graph/graph.html` |
| gleam-website (Gleam, node, esbuild) | `just site`, `just site-serve`, `just deploy-site` | The static site in `site/`: the home of piabrota.art and alerts under `/alerts/`, on Codeberg Pages |

- The Desmos support stays: `just wave` (the Rust binary `wave`, crate `alerts-drawings`)
  writes `wave.svg` and `wave.desmos.json`, and `just desmos-gif` records a Desmos animation
  as a GIF (crate `alerts-desmos`).
  Archify draws diagrams; it does not replace the drawing.
- Every Archify figure has an English twin, `docs/archify/en/<slug>.json`, with the same lanes,
  nodes and edges. `README.pt-BR.md` embeds `docs/img/<slug>.png`; `README.md` embeds
  `docs/img/en/<slug>.png`.
- Commit the JSON, `docs/archify/figures.lock` and the PNG. Never commit `build/`.
- The site keeps its Gleam code small: one module that prints the page. The hosting
  (Codeberg Pages, the DNS of `piabrota.art`) is in `docs/HOSPEDAGEM.md`.
