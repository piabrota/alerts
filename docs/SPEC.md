<!-- reading-time: 4 min -->
# Specification

`spec/` holds the Agda specification of `alerts`: what an alert is, what the harness does to it,
and the facts that hold for every input. The Haskell source in `haskell/` implements it, and its
test suite replays every theorem on the code.

```bash
just spec      # agda type-checks spec/Everything.agda (--safe: no postulate)
just hs-test   # cabal test in haskell/alerts-core: one check per theorem
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

The names follow the z3 statements of [`PROOFS.md`](PROOFS.md) and the checks of
`crates/alerts-proofs`. z3 proves a statement over a model; Agda proves it over the definitions.

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

## Not yet specified

P4 (feed times) and P5 (the rule algebra) are about `alerts-rss`. They enter the spec with the
Haskell port of that crate.

## Port stages

The Rust stays until each part has its Haskell port and its spec. One PR per stage:

1. `alerts-core` and the CAP identifier: this spec, `haskell/alerts-core`.
2. `alerts-rss`: the rule algebra and the feed times, with P4 and P5 in Agda.
3. `alerts-feed`: the Atom + CAP 1.2 channel.
4. The sources: `alerts-source-cemaden`, `alerts-source-mock`, `alerts-catalog`.
5. The service `alerts` (Discord, the feed, `just marola-discord`) and `alerts-mcp`.
6. `alerts-crud` and the desktop app. ⚠ Tauri and Leptos have no Haskell twin: this stage needs a
   decision.
7. `alerts-py` (PyO3) and `alerts-obsidian` (wasm32). ⚠ A pip package and a WebAssembly plugin from
   Haskell need a decision: the GHC wasm backend, or keep these two in Rust over the Haskell core.
