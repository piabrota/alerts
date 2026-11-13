<!-- reading-time: 4 min -->
# Specification

`spec/` holds the Agda specification of `alerts`: what an alert is, what the harness does to it,
and the facts that hold for every input. The Haskell source in `haskell/` implements it, and its
test suite replays every theorem on the code.

```bash
just spec      # agda type-checks spec/Everything.agda (--safe: no postulate)
just test      # cabal test: one check per theorem, and every other test
```

## Three layers

| Layer | Where | What it shows |
|---|---|---|
| Specification | `spec/Alerts/Core.agda` | The definitions: `Uf`, `Severity`, `Alert`, `Stage`, `run`, `fromIbge`, `identifier` |
| Proofs | `spec/Alerts/Properties.agda` | Each fact, for every input, checked by Agda |
| Implementation | `haskell/alerts-core` | The same names in Haskell; `test/Spec.hs` replays each fact on the code |

The Agda is the reference. The Haskell is a separate program, not extracted from the Agda, so the
link between the two is the test suite: exhaustive over the finite domains (27 UFs, 4 severities,
100 two-digit prefixes), and over every text of up to 3 characters of `a`, `-` and `1` for the
identifier.

## The theorems

The names follow the z3 statements (P1 to P6) and the exhaustive checks (C1 to C7) of the previous
implementation: see [`PROOFS.md`](PROOFS.md). z3 proved a statement over a model; Agda proves it
over the definitions.

| Agda theorem | z3 / check | Says |
|---|---|---|
| `run-empty` | — | The empty harness passes every alert |
| `run-++` | — | Composing harnesses is Kleisli composition |
| `keepIf-commute` | C4 | Two stages that only keep or drop commute |
| `stages-commute` | P3 `stages_commute`, C4 | `onlyUfs` and `minSeverity` commute |
| `keepIf-same` | — | A kept alert is the same alert |
| `parity` | P3 `channel_parity`, C5 | Channels over one harness deliver the same list |
| `rank-injective` | C3 | The severity order is strict |
| `upward-closed` | P2 `upward_closed`, C3b | A floor that passes `s` passes every severity above `s` |
| `minor-passes-all` | P2 `minor_passes_all` | The lowest floor drops nothing |
| `ufOfPrefix-sound`, `ufOfPrefix-complete` | P1 | A prefix routes to the UF whose code it is |
| `routing-injective` | P1 `routing_injective`, C2b | No UF is reached by two prefixes |
| `south-is-v0` | P1 `south_is_v0`, C2 | The prefixes 41, 42, 43 are `v0` |
| `seven-is-two` | C1 | A 7-digit code routes as its 2-digit prefix |
| `non-digit-unrouted`, `wrong-length-unrouted` | P1, C1b | A malformed code never routes |
| `identifier-clash` | P6 `identifier_unique` (sat), W6 | `cemaden` + `ped-1` and `cemaden-ped` + `1` give one identifier |
| `identifier-unique` | P6 `identifier_unique_when_ids_start_with_a_digit` | Unique when no source id has `-` before a digit and every alert id starts with a digit |

`parity` holds by construction: a channel is `deliver h`, a function of the one harness output.
The theorem records the rule of `GROUND_TRUTH.md` ("never filter or rewrite alerts per channel")
as a type. `identifier-unique` is about the identifier before the CAP replacement (`capSafe`), as
in the z3 statement: the ids it covers have no character that CAP forbids.

## Tested, not yet proved

P4 (the feed times) and P5 (the rule algebra) are tested on the Haskell code, not yet proved in
Agda. `haskell/alerts/test/Main.hs` checks the calendar for every day of 1900 to 2100 (P4
`civil_round_trip`), the set laws of `union`, `intersect`, `diff` and `filter` (P5), and replays
the counterexamples W1 to W6 (`PROOFS.md`). An Agda model of `Alerts.Rss.Expr` and `Alerts.Time`
is the next step.
