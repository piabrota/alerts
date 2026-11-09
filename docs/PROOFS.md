<!-- reading-time: 6 min -->
# Proofs

`alerts-proofs` writes the alert domain as RDF and its invariants as SMT conjectures, from one
set of facts read out of `alerts-core` at run time — so the proofs cannot drift from the code.
The outputs live in `docs/proofs/` and are committed, like the README drawing: `just proofs`
writes them, `just proofs-check` (in `just check`) fails when they are not the output of the
code, and `just prove` runs z3 on the conjectures.

| File | What it is |
|---|---|
| `docs/proofs/facts.ttl` | The domain as RDF 1.1 Turtle: UFs with their IBGE prefixes, severities with their order, the v0 scope, the sources (CEMADEN, Defesa Civil IDAP) with their area encoding, and the channels. |
| `docs/proofs/p1-routing.smt2` | IBGE routing, as `Uf::from_ibge` does it: total on valid prefixes, visible when unrouted, deterministic, and v0 lands in the South. |
| `docs/proofs/p2-severity.smt2` | The `min_severity` stage is monotone: it never passes a lower severity while dropping a higher one. |
| `docs/proofs/p3-stages.smt2` | `only_ufs` and `min_severity` commute, and two channels that read the same harness output deliver the same alerts (the parity hard rule). |
| `docs/proofs/p4-time.smt2` | The feed times of `alerts-rss` and the Obsidian plugin: the calendar arithmetic is exact for every integer day, and four inputs that the parsers accept and should not. |
| `docs/proofs/p5-rules.smt2` | The laws of the rule algebra of `alerts-rss`, key by key, and the alert predicates. One law of the module comment fails. |
| `docs/proofs/p6-identifier.smt2` | The CAP identifier of `alerts-feed`: unique only when the source ids and the alert ids keep a form. |

## How a property is checked, twice

1. **In Rust, exhaustively.** The domains are finite (100 prefixes, 4 severities, 27 UFs), so
   `alerts-proofs` runs every case against the real `alerts-core` functions — `Uf::from_ibge`,
   `min_severity`, `only_ufs`, `Harness::run` — and names the first counterexample. This is the
   gate: it runs in `just proofs-check`, with no tool beyond cargo.
2. **In z3, by refutation.** Each `.smt2` file asserts the negation of a statement inside a
   `(push)/(pop)` block. `just prove` (`proofs --prove`, `refute.rs`) runs z3 on each block
   alone and prints one line: `OK` or `FAIL`, the name, the time, and the counterexample when
   there is one. `unsat` proves a statement. A statement marked `expect: sat` must have a
   counterexample: it shows that a hypothesis is needed, or that the code allows more than its
   comment says. z3 is in the dev shell. The method is the one of `z3/refute.py` in
   [h0ffmann/proof](https://github.com/h0ffmann/proof).

## Statements about the code (P4 to P6)

P1 to P3 state properties of a model of the pipeline. P4 to P6 (`crates/alerts-proofs/src/code.rs`)
restate the arithmetic and the logic of named functions, with their `file:line`, for every
input. A test shows that the code works on the inputs that ran; a statement here holds for all
of them, or z3 finds the input where it fails. Every counterexample is replayed through the real
Rust in `checks.rs` (W1 to W6, in `just proofs-check`), so it is a fact about the code and not
only about its model.

```sh
just prove      # 38 statements, about 40 s; exit code 1 if one fails
```

| Statement | Code | What z3 shows | Theory |
|---|---|---|---|
| `civil_round_trip` | `alerts-rss/src/time.rs:15-41` | `days_from_civil` undoes the civil date of `rfc3339_utc`, for every integer day | integers |
| `civil_is_a_calendar_date` | `time.rs:15-29` | the month is 1 to 12 and the day fits the month, leap years included | integers |
| `days_strictly_monotone` | `time.rs:32-41` | a later date has a larger day number | integers |
| `utc_text_order_is_time_order` | `time.rs:1-4` | from the three above: a later day prints a later date | uninterpreted functions |
| `obsidian_epoch_agrees` | `alerts-obsidian/src/active.rs:34-42` | the plugin counts the same days as `alerts-rss` | integers |
| `union_*`, `intersect_*`, `diff_*`, `filter_filter`, `filter_of_diff` | `alerts-rss/src/expr.rs:5-8, 187-266` | the set laws of the module comment, key by key | datatypes |
| `filter_union_when_items_agree` | `expr.rs:9` | the law of line 9 holds when `p` gives one answer on the two items of a shared key | datatypes |
| `ufs_any_is_union`, `severity_all_is_max`, `severity_any_is_min` | `expr.rs:329-339` | `any`, `all` of the alert predicates are one predicate | bit-vectors, integers |
| `identifier_unique_when_ids_start_with_a_digit` | `alerts-feed/src/lib.rs:65-68` | the identifier is unique when no source id has a `-` before a digit and every alert id starts with a digit; C7 checks the catalog | quantifiers |

Seven statements expect a counterexample. Each one is a gap between the code and its comment, or
a hypothesis that a law needs:

| Statement | Counterexample | Replay |
|---|---|---|
| `accepted_dates_are_dates` | `epoch` (`time.rs:45-46`) takes `1 <= d <= 31` for every month: `2026-02-31` becomes `2026-03-03` | W1 |
| `accepted_clock_within_the_day` | `23:59:60` is accepted and is `00:00:00` of the next day | W2 |
| `offset_within_a_day` | the offset (`time.rs:76-84`) is not range-checked: `+99:99` is accepted, and `+00:60` equals `+01:00` | W3 |
| `utc_year_has_four_digits` | the year `10000` is accepted; `10000-01-01T…` sorts before `2026-…`, so `since` and the canonical order misjudge it | W4 |
| `filter_union_distributes` | the law of `expr.rs:9` fails when two feeds hold one key: if the winner fails `p` and the loser passes, `filter(p, union(a, b))` drops the key and `union(filter(p, a), filter(p, b))` keeps the loser | W5 |
| `ufs_all_is_intersection` | an alert for PR and SP passes `all[ufs [PR], ufs [SP]]`, but `ufs []` keeps nothing | none: a fact about the predicates, not a gap |
| `identifier_unique` | `identifier` (`alerts-feed/src/lib.rs:65-68`) joins with `-`, a letter of source ids: `cemaden` with the id `ped-1` and `cemaden-ped` with the id `1` give one identifier. Both sources are in the catalog. CEMADEN ids start with a digit (`<cod_alerta>-<stamp>`), so v0 is safe | W6 |

What this does not cover: `map`, `dedupe` and `take` (they do not act key by key); the
replacement of the characters that CAP forbids in `identifier` (z3's string solver gives
`unknown` on it, and it maps both ` ` and `,` to `_`); the RFC 822 parser apart from its shared
`epoch`. z3's string solver also cannot close the uniqueness argument of P6, so that statement
runs over the characters of the text as an uninterpreted function.

## Why RDF

The Turtle file is the machine-readable statement of what the system believes: which agency is
official, how a source names an area, which UFs v0 serves. A SPARQL store, a SHACL shape or the
wiki can read it; the SMT files are one consumer among several. Both sources of v0 — the CEMADEN
municipal alerts and the Defesa Civil IDAP CAP feed — name areas with IBGE municipality codes,
which is why P1 (routing by IBGE prefix) is the first proof.

## What this is not

P1 to P3 state properties of the *model* of the pipeline, observed from the code at generation
time; the exhaustive Rust checks cover that gap for the finite domains. P4 to P6 restate the code
by hand: a change to a function named there needs a change to its statement, and the replays W1
to W6 fail when a counterexample no longer holds. Verifying `alerts-core` functions directly (for example with Kani) is a
possible next step, out of scope here.
