<!-- reading-time: 2 min -->
# Proofs

The proofs of `alerts` are the Agda specification in `spec/`: read [`SPEC.md`](SPEC.md). Agda
proves each fact over the definitions, for every input, and the Haskell test suites replay each
fact on the code.

The previous implementation had z3 statements (P1 to P6, in `docs/proofs/`) and exhaustive checks
(C1 to C7). Their names stay as the names of the Agda theorems and of the tests, so a reader of an
old PR can find each one. The z3 files left with the Rust code: the Agda theorems prove P1 to P3
and P6 for every input, which is stronger than the model that z3 checked.

## The counterexamples (W1 to W6)

Six statements of the old proofs had a counterexample. Each one is a gap between the code and its
comment, or a hypothesis that a law needs. `haskell/alerts/test/Main.hs` (`witnesses`) replays
each one on the Haskell code, so a fix shows up as a failing check.

| Check | Counterexample |
|---|---|
| W1 | The RFC 3339 parser takes `1 <= d <= 31` for every month: `2026-02-31` becomes `2026-03-03` |
| W2 | `23:59:60` is accepted and is `00:00:00` of the next day |
| W3 | The offset is not range-checked: `+99:99` is accepted, and `+00:60` equals `+01:00` |
| W4 | The year `10000` is accepted. `10000-01-01T…` sorts before `2026-…`, so `since` and the canonical order misjudge it |
| W5 | `filter` does not distribute over `union` when two feeds hold one key: if the winner fails `p` and the loser passes, `filter(p, union(a, b))` drops the key and `union(filter(p, a), filter(p, b))` keeps the loser |
| W6 | `identifier` joins with `-`, a letter of source ids: `cemaden` with the id `ped-1` and `cemaden-ped` with the id `1` give one identifier. CEMADEN ids start with a digit, so v0 is safe. In Agda: `identifier-clash` |

C7 checks the hypothesis of `identifier-unique` over the catalog: no source id has a `-` before a
digit.
