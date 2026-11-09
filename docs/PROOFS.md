<!-- reading-time: 2 min -->
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

## How a property is checked, twice

1. **In Rust, exhaustively.** The domains are finite (100 prefixes, 4 severities, 27 UFs), so
   `alerts-proofs` runs every case against the real `alerts-core` functions — `Uf::from_ibge`,
   `min_severity`, `only_ufs`, `Harness::run` — and names the first counterexample. This is the
   gate: it runs in `just proofs-check`, with no tool beyond cargo.
2. **In z3, by refutation.** Each `.smt2` file asserts the negation of a conjecture inside a
   `(push)/(pop)` block. `just prove` requires every `(check-sat)` to answer `unsat`; a `sat`
   answer carries a counterexample model. z3 is in the dev shell.

## Why RDF

The Turtle file is the machine-readable statement of what the system believes: which agency is
official, how a source names an area, which UFs v0 serves. A SPARQL store, a SHACL shape or the
wiki can read it; the SMT files are one consumer among several. Both sources of v0 — the CEMADEN
municipal alerts and the Defesa Civil IDAP CAP feed — name areas with IBGE municipality codes,
which is why P1 (routing by IBGE prefix) is the first proof.

## What this is not

The SMT conjectures state properties of the *model* of the pipeline, observed from the code at
generation time. They do not verify the Rust itself; the exhaustive Rust checks cover that gap
for the finite domains. Verifying `alerts-core` functions directly (for example with Kani) is a
possible next step, out of scope here.
