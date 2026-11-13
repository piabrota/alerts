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
- **South extensions in `forks/`:** `forks/br_south_disaster` and `forks/br_south_waterq` are git
  submodules pinned to a commit, and they are the planned sources of `alerts` for the South. The
  path is the extension id, because the host loads an extension only from a directory with that
  name. Their Python stays in their own repositories: this repo holds the submodule links, never a
  copied `.py` file. The shared module that the Rust gave them (`piabrota_alerts.ext`) left with
  the Rust: no extension used it, and a Haskell replacement needs a decision ⚠.
  `docs/br-south/index.html` shows how they load. `just forks-init` gets them.
- **Codeberg mirror without AI trace:** `codeberg.org/piabrota/alerts` holds the code with no
  trace of AI tools. No agent file (`AGENTS.md`, `verbs/` as skills, `scripts/`, `opencode.json`,
  `.mcp.json`), no AI co-author trailer, and no AI tool name in a commit message. Build the copy
  only with `just export-codeberg <out-dir>` (Haskell, `scripts/export-codeberg/`): it fails when a
  trace remains.
  Commit the result with a plain message as the author. Never push this repo's history there.

## Hard rules

- **Haskell is the source, Agda is the specification.** Every program, script and tool of
  `alerts` is Haskell (`haskell/`, `scripts/export-codeberg/`), and its meaning is the Agda
  specification in `spec/` (`--safe`, no postulate). A Haskell module states which Agda
  definitions it implements, and its test suite replays the theorems of the spec on the code, one
  check per theorem name (`just spec`, `just test`, in `just check`). A change of behavior changes
  the spec first. Never commit a `.rs` or a `.py` file. A shell recipe in the `justfile` may call
  tools, but a recipe that needs logic becomes a Haskell program.
- **No mobile and no desktop app.** `alerts` is a service, a feed and an MCP server. The Rust
  desktop app, the Obsidian plugin, the pip package and the app store plans left with the Rust; a
  new interface needs a human decision first.
- **No new CI.** `.github/workflows/haskell.yml` builds and tests the Haskell; do not add another
  workflow. `just check` is the gate, run by hand before a PR.
- **Every PR delivers a test alert to the official Marola Discord.** `just marola-discord` (in
  `just check`) posts one test alert (CAP status Test, headline `TESTE: ...`) to the webhook in
  `MAROLA_DISCORD_WEBHOOK_URL`. It fails when the variable is missing, when the URL is not a Discord
  webhook, or when Discord does not store the message. Write the result in **Testado**. Never
  print the webhook URL: it holds the token. The setup is in `docs/DISCORD.md`.
- **Every PR uses the `open-pr` verb.** The title is pt-BR. The body is bilingual, pt-BR first, at
  the `estrito` level of `write-pt-br`, and ends with `verbs/open-pr/footer.md`. One PR per request:
  new work for an open PR goes into that PR.
- **A PR that changes an image uses the `open-pr-images` verb** (bipri): the `open-pr` body plus
  before and after images, committed in `docs/screenshots/<topic>/`.
- **Identify honestly.** Every HTTP request goes through `Alerts.Http` and sends `Alerts.Core.userAgent`
  (`piabrota-alerts/<version> (+repo URL)`). Never send a browser User-Agent and never copy another
  client's name, not even as a fallback. When a source refuses the honest name, stop and tell a
  human: the fix is to ask the agency, not to disguise the client.
- **FOSS first.** A new dependency must carry one of these licenses: MIT, Apache-2.0, MPL-2.0,
  BSD-2-Clause, BSD-3-Clause, ISC. Check it on Hackage before you add it, and add it to the
  `ghcWithPackages` list of `flake.nix`, so `cabal --offline` builds from the pinned nixpkgs. Do not
  use paid resources or paid APIs without explicit human confirmation.
- **No secrets in the repo.** The Discord webhooks come from `DISCORD_WEBHOOK_URL` (the alerts
  server) and `MAROLA_DISCORD_WEBHOOK_URL` (the official Marola server). The source tokens are BYOT:
  each person brings their own in `ALERTS_TOKEN_<ID>` (`Alerts.Catalog` names the variable). They
  live in the environment or in `.env`, which git ignores; only the empty `.env.example` is
  committed. Never print a token: `Alerts.Tokens.Token` hides it in `Show`.
- **Official sources only.** An alert comes from an authoritative agency. Never invent an alert,
  an area, a time or an action that the source does not state.
- **No partial functions** (`head`, `fromJust`, `error`) outside tests, and no `unsafePerformIO`.
  `just build` builds with `-Wall -Werror`.
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
just check         # format, build (-Werror), tests, Agda, stale translations, the Marola test
just spec          # type-check the Agda specification
just test          # cabal test all: the spec theorems on the Haskell code, the service, the export
just test-message  # publish one test alert to DISCORD_WEBHOOK_URL and/or FEED_PATH
just marola-discord # the hard-rule test: one test alert to MAROLA_DISCORD_WEBHOOK_URL
just mock-alerts    # deliver the mock alerts (TESTE) to DISCORD_WEBHOOK_URL and/or FEED_PATH
just poll          # read the sources of ALERTS_SOURCES (for example cemaden) and deliver new alerts
just sources       # search the catalog of official sources; BYOT tokens in ALERTS_TOKEN_<ID>
just mcp           # run the MCP server on stdio
```

## Repo layout

The Agda specification is `spec/` (`spec/Alerts/Core.agda`, `spec/Alerts/Properties.agda`).
`docs/SPEC.md` maps each theorem to its Haskell check. The Haskell source is a cabal project
(`cabal.project` at the root), one package per directory:

| Package | Kind | Role |
|---|---|---|
| `haskell/alerts-core` | lib | Domain types (`Alert`, `Uf`, `Severity`, `Hazard`), the FP harness and the CAP identifier: the twin of `spec/Alerts/Core.agda`. Knows nothing about Discord, HTTP or MCP. `test/Spec.hs` replays every theorem of the spec. |
| `haskell/alerts` | lib + 2 exe | The service and its parts, one module each: `Alerts.Feed` (the Atom + CAP 1.2 channel; a test alert is always `<status>Test</status>`), `Alerts.Rss.*` (RSS and Atom items, the rule algebra and its evaluator, `docs/RSS.md`), `Alerts.Time` (feed times), `Alerts.Catalog` (the catalog of official sources, BYOT), `Alerts.Source.Cemaden` and `Alerts.Source.Mock` (the sources; an unknown field fails loudly), `Alerts.Store` (SQLite, `ALERTS_DB`), `Alerts.Discord`, `Alerts.Http` (the honest User-Agent), `Alerts.Service` (the poll). Executables: `alerts` (the CLI) and `alerts-mcp` (MCP server, stdio JSON-RPC, configured in `.mcp.json` and `opencode.json`). Only `Alerts.Http`, `Alerts.Store`, `Alerts.Env` and the executables do I/O. |
| `scripts/export-codeberg` | exe | `export-codeberg`: the Codeberg copy without AI trace. |

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

**Sources are modular.** Brazilian data is scattered, so each source is its own module,
`Alerts.Source.<Id>`, that turns the agency's data into `Alert`s, and one constructor of
`Alerts.Service.Source`. Adding a source, or a state, adds a module. It does not change the core.

## The FP harness

The design follows the Object-Oriented Harness paper (arXiv 2610.28380) and translates it to
functional composition. A position paper on the FP harness lives in the `smart-notes` repo.

| OO Harness | alerts |
|---|---|
| A subclass overrides a method | A new `Stage` wraps the pipeline: `andThen` |
| `super()` | The stages before it in the composition |
| Inheritance chain | Kleisli composition (`foldr (>=>) Just` over the stages) |
| Router picks a class | A pure function that matches on an enum and returns a `Harness` |
| Fall back to the base harness | `Maybe`/`Either` falls back to the base harness |

A `Stage` is `Alert -> Maybe Alert`: pass the alert on, possibly changed, or drop it.
Specialize by composing a stage. Never add inheritance-like class hierarchies.

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

## Apps

An app is one deployment: sources, a harness and channels for one audience. Read the skills
before you work on these parts:

| Skill | Job |
|---|---|
| `manage-apps` | What an app is, where its parts go |
| `generate` | New source, stage or channel |
| `diagnose` | Check that an app can run |
| `schedule` | Poll rates, delivery once per id, one-off tasks |
| `map` | Alert areas as GeoJSON from CAP 1.2, FOSS tiles only |

The store has no interface: `alerts feed-add` and `alerts rule-add` write the feeds and the rules,
and no command creates an alert.

## Why Haskell and Agda

The choice is for a compiler, safety and long-term robustness (antifragility), with a proof of
what the code means. Haskell gives algebraic types, pattern matching and pure functions by default:
the FP harness is Kleisli composition (`>=>`) in the language itself. Agda states the semantics and
proves the invariants for every input, where tests cover only the inputs that ran. The Haskell keeps the names of the Agda definitions, so a
reader can put a theorem next to the code it is about.

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

- The README drawing (`docs/drawings/wave.svg`, `wave.desmos.json`) and the Desmos GIF stay as
  files. Their Rust generators left with the Rust. Archify draws diagrams; it does not replace the
  drawing.
- Every Archify figure has an English twin, `docs/archify/en/<slug>.json`, with the same lanes,
  nodes and edges. `README.pt-BR.md` embeds `docs/img/<slug>.png`; `README.md` embeds
  `docs/img/en/<slug>.png`.
- Commit the JSON, `docs/archify/figures.lock` and the PNG. Never commit `build/`.
- The site keeps its Gleam code small: one module that prints the page. The hosting
  (Codeberg Pages, the DNS of `piabrota.art`) is in `docs/HOSPEDAGEM.md`.
