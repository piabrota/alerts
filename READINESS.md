<!-- reading-time: 6 min -->
# READINESS: how far alerts is from release 0

Checkpoint of 2026-10-27, at `main` `1a90040`. This page compares the repo with the v0 goal in
`GROUND_TRUTH.md`. Update it at each checkpoint: change a status only after you check it, and keep
each `⚠`.

## Verdict

**Not ready.** The tooling, the channels and the interfaces are in place. The product is not: no
v0 source is implemented, so the service can only send a test message. Release 0 needs at least
one real source, a polling loop with delivery once per id, and a place where the service runs.

| Area | Status |
|---|---|
| Tooling, docs, licenses, Codeberg export | done |
| Channels (Discord, Atom + CAP 1.2) | done for the test message |
| Python package, MCP server, desktop app | done as a skeleton |
| v0 sources (Defesa Civil, INMET, CEMADEN) | **missing** |
| Service loop: poll, store, deliver once per id | **missing** |
| Hosting of the service and the feed | **missing** |
| Site and email at `piabrota.art` | blocked on a person |
| Codeberg mirror sync | in progress (PR #19) |
| Hard rule "No CI" | **broken** (`.github/workflows/rust.yml`) |

Legend: **done** (on `main`), **in progress** (an open PR), **blocked** (waits on a person or a
choice), **missing** (nobody started it).

The app stores (App Store, Google Play) have their own map: [`docs/STORES.md`](docs/STORES.md). It starts
where this page stops.

## Done

- **Rename to `piabrota/alerts`** (#21, on `main` as `cbe5580`). Crates `alerts-*`, binary
  `alerts`, pip package `piabrota-alerts`, new User-Agent, new Codeberg mirror name.
- **Core and harness** (`alerts-core`). `Alert`, `Uf`, `Severity`, `Hazard`, and the FP harness
  (`Harness::then`, `only_ufs`, `min_severity`). `Uf::V0` is PR, SC, RS.
- **Discord channel** (`alerts test-message`). One embed with pt-BR labels and a color per
  severity. The webhook comes from `DISCORD_WEBHOOK_URL`.
- **Atom + CAP 1.2 channel** (`alerts-feed`). The test alert is `<status>Test</status>`, and the
  feed file is written atomically.
- **Python package** (`alerts-py`). The wheel builds with maturin, and `import piabrota_alerts`
  exposes `v0_ufs` and `parse_uf`.
- **MCP server** (`alerts-mcp`). Two tools: `v0_ufs` and `check_alert`.
- **Desktop app** (#22, on `main` as `1a90040`). Tauri 2 + Leptos, Loco-style CRUD
  (`alerts-crud`, SQLite) for sources, channels, apps and alerts, a `scaffold` generator, and a
  map page (MapLibre + OpenFreeMap). See `docs/DESKTOP.md`.
- **Static site** (#18). Gleam site in `site/` and the recipe `just deploy-site`. The deploy never
  ran (see issue #20 below).
- **Trails raw-source tool** (#17). The binary `trilhas` converts a Wikiloc GPX to a row of
  `data/raw/trilhas/trilhas.tsv`. `praias.tsv` has 51 beaches; nobody checked the list against a
  source ⚠.
- **Repo hygiene.** `just check` (fmt, clippy, tests, cargo-deny, REUSE, stale translations,
  pinned tools, figures, reading time, desktop). `just export-codeberg` builds a copy with no AI
  trace. Docs in pt-BR and EN, wiki in four languages in `wiki/`. About 35 unit tests.
- **Source research** (`docs/SOURCES.md`). Endpoints, formats and access checked on 2026-10-22.

## In progress

- **PR #19, `sync-codeberg`.** Publishes the clean copy to `codeberg.org/piabrota/alerts`.
  Green, no conflict, waits for your merge. After the merge, a person runs the first sync, because
  the agent network blocks codeberg.org.

## Blocked

| Item | Waits on | Next step |
|---|---|---|
| Issue #20: site at `piabrota.art` and `/alerts/`, Porkbun email forwarding | a person (codeberg.org is blocked in the agent network) | Create `codeberg.org/piabrota/pages`, add the DNS records of the issue, run `just deploy-site`, turn on email forwarding |
| Trails in `trilhas.tsv` (the file has only the header) | a choice: export the GPX files from Wikiloc by hand, or use OpenStreetMap | Wikiloc has no API and is unreachable from the agent network. OSM (ODbL) can be fetched by a tool; the choice is yours |
| Issue #1: GitHub Wiki | the repo is public now ⚠ (said in the rename work; check the settings) | Turn on the Wiki, create the first page, run `just wiki-sync` |
| INMET with the new User-Agent | a person on a network that reaches INMET | Run one request with `piabrota-alerts/0.0.0`; if INMET refuses, ask the agency (hard rule) |

## Missing for release 0

In order of need. The first three are the release.

1. **v0 source crates.** No `crates/alerts-source-<id>` exists. Start with Defesa Civil IDAP: it
   is already Atom + CAP 1.2, so the parse is the smallest. Then CEMADEN `wsAlertas2` and INMET
   `avisos/ativos`. Each undocumented endpoint must fail loudly. Record real alerts as test cases.
2. **Service loop.** A `run` command in `alerts`: poll each source at its rate (IDAP keeps only
   about 12 hours ⚠), store what it read, and deliver each alert id once to every channel. The
   `schedule` skill describes the rates; nothing implements them.
3. **Hosting.** Where the service runs, and where the Atom feed is served. Nothing is chosen. It
   must be free or have your confirmation (hard rule "FOSS first").
4. **Flood harness for v0.** A composed `Harness` with `only_ufs(Uf::V0)` and a hazard filter for
   floods, used by the service. The stages exist; the v0 composition does not.
5. **CAP validation.** `GROUND_TRUTH.md` says the CAP output must validate against the OASIS CAP
   1.2 XSD. No test does this ⚠.
6. **Area geometry.** `Alert` has the area only as text, so the map draws no shape. A geometry
   field in the core is the next step for the map.
7. **Release mechanics.** The version is `0.0.0` in `Cargo.toml` and `CITATION.cff`, and there is
   no tag. Release 0 needs a version, a tag, and the README status changed from "bootstrap".

Not in v0: Indexgram (issue #8), the planned sources (ANA, CPTEC/INPE, Epagri/Ciram, SGB,
Defesa Civil RS, SIMEPAR), and the local LLM stages (Ollama, OLLAYA).

## Rules that the repo breaks now

- **"No CI" is broken.** `.github/workflows/rust.yml` arrived with commit `7da7139` ("Create
  rust.yml"), outside a PR. It runs `cargo build` and `cargo test`, then a `Typeform/siesta` step
  with empty Google credentials. Decide: delete the workflow and keep `just check` as the gate, or
  change the rule in `GROUND_TRUTH.md`. The green checks on PR #19 come from this workflow.
- **Issue texts use old names.** Issues #1 and #8 still say `smart-alerts` and `AGENTS.md`. They do
  not block the release.

## Checklist for release 0

- [ ] Decide on `.github/workflows/rust.yml`
- [ ] Merge PR #19 and run the first Codeberg sync
- [ ] Source crate: Defesa Civil IDAP, with recorded test cases
- [ ] Source crates: CEMADEN and INMET
- [ ] `alerts run`: poll, store, deliver once per id, to Discord and the feed
- [ ] v0 flood harness used by the service
- [ ] CAP 1.2 XSD validation test
- [ ] Choose and set up hosting for the service and the feed
- [ ] Check INMET with the new User-Agent
- [ ] Issue #20: site and email at `piabrota.art`
- [ ] Version `0.1.0` (or `0.0.1`), tag, README status, `CITATION.cff`
