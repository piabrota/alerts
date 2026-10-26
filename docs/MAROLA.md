# marola: planned alert work, and what it means for smart-alerts

[marola](https://github.com/marola-dev/marola) is an ocean intelligence layer: it scores the sea
for swimmers on the Brazilian coast. Its design documents are MIPs (Marola Improvement Proposals)
in `docs/MIPs/`. Several MIPs plan alert work. This page lists them, compares them with
smart-alerts, and lists what each project can take from the other.

Read on 2026-10-22, from these versions:

- `marola-dev/marola` `main` at `1f46fae9` (2026-10-21).
- PR #542 (MIP-0071, open) at `08d797a5`.
- PR #669 (MIP-0034 §5.2a, open) at `ec35745a`.

No alert code exists in marola yet. Every MIP below is `Draft`. The claims in the "marola
plans" sections are marola's claims. This page did not check them again, except where it says so.

## The MIPs

| MIP | Title (short) | Status | Alert content |
|---|---|---|---|
| MIP-0034 | RSS in and RSS out | Draft, on `main` | §5.2: an INMET alert banner in the CLI and on the map. A task list for the alert arm (`MIP-0034.tasks.md`). |
| MIP-0034 §5.2a | The alerts archive page | open PR #669 | An `alerts.html` archive per state, stored in a DuckLake on Cloudflare R2, with a check of each expired alert against Open-Meteo. |
| MIP-0071 | Official alerts per state | open PR #542 | An append-only JSONL store per UF and month, an hourly ingest, an RJ backfill for September 2026, and `alerts.html`. |
| MIP-0062 | Ressaca hazard | Draft, on `main` | A deterministic storm-surge veto in the swim score. It uses the Navy threshold, not a Navy alert. |
| `FUTURE-WORK.md` §9.2 | A hazard detection agent | candidate | A proactive alert agent. It is gated: a human-confirmation design must come first. |

MIP-0071 and MIP-0034 §5.2a both design the alerts page, with two different stores. Both PRs are
open. marola has not chosen one of them yet.

## What marola plans

### Source: INMET only

- RSS is the index: `https://apiprevmet3.inmet.gov.br/avisos/rss`.
- CAP 1.2 is the content: one document per alert at `/avisos/rss/<id>`.
- `/avisos/ativos` (JSON) is not used. It is undocumented. A later change can add it, but it must
  fall back to RSS and CAP.
- Old ids still serve full CAP: 40000 is from 2022-08-29. `/avisos/rss/1000` returns `500`.
  A backfill can walk the ids downward ⚠ (only five ids were sampled).
- The Navy (Marinha do Brasil, CHM) is the best marine source. It answers `403` (Cloudflare) and
  has no feed. marola rejects a headless-browser scraper and plans to ask the Navy for a feed.
- Rio state and city civil defence (Alerta Rio, COR, Defesa Civil RJ) answered `403` to marola.
  They are candidate adapters ⚠.

### Model and rules

- Severity has two vocabularies. CAP gives `Moderate` / `Severe` / `Extreme`, and it sets the
  order. INMET gives `Perigo Potencial` / `Perigo` / `Grande Perigo`, and marola shows it
  verbatim. marola never computes the Portuguese label from the CAP value.
- The area match is geometry: a point-in-polygon test against the CAP `<polygon>`. A
  hand-written mesoregion table was removed (PR #314).
- MIP-0071 routes an alert to a UF by the IBGE municipality code (the first two digits). It
  falls back to mesoregion names, then to an `_unrouted/` file, so a gap is visible.
- The banner keeps an event allow-list for the sea: `Ventos Costeiros`, `Vendaval`,
  `Tempestade`, `Chuvas Intensas`. The archive keeps every event.
- An alert shows only inside `[onset, expires]`.
- MIP-0034 task 2 "fails soft": a fetch error gives an empty list (`Nil`) and no exception.
  Task 3 prints `⚠ INMET — no active warning for this area` when the list is empty. So the CLI
  shows a failed fetch as "no warning". MIP-0071 is better here: it records the last error per UF
  in `status.json`, and the page shows the time of the last successful ingest. This conflicts
  with smart-alerts (see "Conflict: a failed fetch").
- No language model is on the alert path. The alert text is the issuer's text, with a source link.
- An alert does not change the swim score. A score change needs its own MIP.
- No push to Telegram or e-mail. A proactive alert waits for the human gate of
  `FUTURE-WORK.md` §9.2.

### Store (two open designs)

| | MIP-0071 (PR #542) | MIP-0034 §5.2a (PR #669) |
|---|---|---|
| Where | `site-data` branch of the repo | DuckLake on Cloudflare R2 (MIP-0075, marola-oods) |
| Shape | `alerts/<UF>/<YYYY-MM>.jsonl`, append-only, plus the raw CAP files | tables `alert` and `alert_check`, partitioned by state and year |
| Dedup | `(key, uf, version)`, `version` = sha256 of the canonical record | ⚠ not stated in the PR |
| Corrections | a changed alert appends a line with `supersedes`; a CAP `Cancel` marks the old alert | ⚠ not stated in the PR |
| Retention | forever | forever |
| First state | RJ, backfilled for September 2026 | RJ |

## Overlap with smart-alerts

| Topic | marola | smart-alerts |
|---|---|---|
| Hazard | the sea: wind, storms, ressaca | floods |
| Region | the coast, RJ first; all 27 UFs in the store | PR, SC, RS |
| Sources | INMET; the Navy later | Defesa Civil (IDAP), INMET, CEMADEN |
| Delivery | a page and a banner; no push | a push to Discord, and an Atom feed of CAP 1.2 alerts with the same text |
| Language | Scala (the app), Python (scripts) | Rust, with a Python package |
| Model | none on the alert path | deterministic harness; a local model only as a `Stage`, later |
| Issuer text | verbatim, with a link | verbatim, `write-pt-br` (`estrito`) |

Both projects use the same rules: official sources only, verbatim issuer text, no invented
field, and loud failure on an undocumented endpoint.

## Conflict: a failed fetch

`GROUND_TRUTH.md` says that a module on an undocumented endpoint must fail loudly, never silently.
MIP-0034 task 2 does the opposite for INMET: a failed fetch gives the same output as "no active
warning". A user then cannot see that a real warning is missing.

**Decision (maintainer, 2026-10-22): the smart-alerts rule wins.** It applies when the two
projects connect: shared code, a shared fixture set, or smart-alerts alerts sent to marola.

- A fetch error is never an empty list. It is an error that a human sees: a log line, a status
  record, and a status message where the alerts go.
- "No active alert" is a claim. smart-alerts makes it only after a successful fetch, and it states
  the source and the fetch time.
- This applies to every source, documented or not. An official endpoint can fail too.
- smart-alerts does not take MIP-0034 task 2's fail-soft rule. MIP-0071's per-UF `status.json`
  is compatible, if its error reaches a human.

## What smart-alerts can take from marola

1. **Two severity vocabularies.** Keep the CAP value for order and the INMET label for display.
   Never derive one from the other. `Severity` in `smart-alerts-core` must keep both.
2. **CAP polygon, not area names.** Match a place against the CAP `<polygon>`. Route to a `Uf`
   by the IBGE code prefix. Keep an "unrouted" bucket so that a gap is visible.
3. **The INMET id walk.** It can give years of real alerts for PR, SC and RS ⚠. These are the
   eval cases that `docs/SURVEY_NLP.md` asks for before any model stage.
4. **An append-only store with a version hash.** The IDAP feed keeps about 12 hours ⚠, so
   smart-alerts must store what it reads. MIP-0071 §5.2–§5.3 is a complete, reviewed design for
   this: dedup key, `supersedes`, `Cancel`, one file per UF.
5. **Test cases.** MIP-0034 tasks 1–2 and MIP-0071 task 2 list tests that apply here too: an
   expired alert is absent, one failed CAP fetch does not lose the other alerts, a document
   without a polygon parses, and a point exactly on a vertex.

## What marola can take from smart-alerts

These are facts that smart-alerts checked on 2026-10-22 (see `docs/SOURCES.md`). They answer
some of the open items in MIP-0071 §4.1.

1. **An honest User-Agent works at INMET.** INMET drops curl's default User-Agent. The
   smart-alerts User-Agent (`smart-alerts/<version> (+repo URL)`) gets `200` on `avisos/rss` and
   `avisos/ativos`. MIP-0071 §4.1 item 5 asks if only a browser-like User-Agent works. The
   smart-alerts result says no, but marola's own User-Agent (`marola/0.1 (+…)`) is not checked ⚠.
2. **`alertas2.inmet.gov.br` resets the connection**, from inside and outside Brazil. MIP-0071
   §4.1 item 6 lists it as a candidate human page per alert.
3. **IDAP is a second official CAP source.** `https://idapfile.mdr.gov.br/idap/api/rss/cap` is an
   Atom feed of CAP alerts from Defesa Civil Nacional and state agencies ⚠ (undocumented). No
   marola MIP mentions it. It can carry ressaca and coastal alerts from a civil defence agency ⚠
   (not seen in a sample). It can replace part of MIP-0071 §4.3.
4. **Fail loudly.** A failed fetch must not look like "no warning" (see "Conflict: a failed
   fetch"). This is a proposal to marola. marola has not accepted it.
5. **A reusable library.** `smart-alerts-py` is a pip package. A marola Python script can use it
   to read official alerts, if marola wants that. Nobody decided this.

## Open questions

1. Can both projects share one CAP parser and one INMET adapter? The languages differ (Scala and
   Rust). A shared fixture set (real CAP documents) is cheaper than shared code.
2. Does IDAP carry coastal events (COBRADE 1.3.1.1.2, storm tide and ressaca) for SC and RJ? ⚠
   Check one month of the feed.
3. Ask the Navy for a feed one time, for both projects. MIP-0034 §11.3 already plans the e-mail.
4. When marola merges PR #542 or PR #669, update the store section of this page.

## ⚠ claims on this page

- The INMET id walk works for all ids and years (marola sampled five ids).
- Rio state and city civil defence sources: format, feed and licence are unknown.
- The dedup and correction rules of PR #669 are not in the PR text.
- The IDAP retention (about 12 hours) and its list of agencies.
- marola's own User-Agent gets a response from INMET.
- IDAP carries ressaca or coastal alerts.
