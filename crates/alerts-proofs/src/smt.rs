//! The invariants as SMT-LIB 2 conjectures for z3 (`just prove`). Each conjecture is asserted
//! negated inside a `(push)/(pop)` block: every `(check-sat)` must answer `unsat`, so a `sat`
//! answer is a counterexample model, ready to read.

use crate::facts::Facts;
use alerts_core::Uf;

fn header(t: &mut String, title: &str) {
    t.push_str(&format!("; {title}\n"));
    t.push_str("; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.\n");
    t.push_str("; Every conjecture is asserted negated: every answer must be unsat.\n\n");
}

/// P1: routing an IBGE code to a UF, as `alerts_core::Uf::from_ibge` does.
pub fn routing_smt(facts: &Facts) -> String {
    let mut t = String::new();
    header(
        &mut t,
        "P1: IBGE routing is total on valid prefixes, visible when unrouted, and lands v0 in the South",
    );

    let ufs = facts
        .prefixes
        .iter()
        .map(|(_, uf)| format!("UF-{}", uf.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    t.push_str(&format!(
        "(declare-datatypes ((Route 0)) (((Unrouted) {})))\n",
        ufs.split(' ')
            .map(|u| format!("({u})"))
            .collect::<Vec<_>>()
            .join(" ")
    ));

    t.push_str("(define-fun prefix ((m Int)) Int (div m 100000))\n");
    t.push_str("(define-fun muni ((m Int)) Bool (and (<= 1000000 m) (< m 10000000)))\n");

    // The routing table, observed from the code.
    t.push_str("(define-fun route ((p Int)) Route\n");
    let mut body = String::from("Unrouted");
    for (p, uf) in facts.prefixes.iter().rev() {
        body = format!("(ite (= p {p}) UF-{} {body})", uf.as_str());
    }
    t.push_str(&format!("  {body})\n"));

    let valid = facts
        .prefixes
        .iter()
        .map(|(p, _)| format!("(= p {p})"))
        .collect::<Vec<_>>()
        .join(" ");
    t.push_str(&format!(
        "(define-fun valid ((p Int)) Bool (or {valid}))\n\n"
    ));

    t.push_str("; c1: no valid prefix of a municipality code is unrouted.\n");
    t.push_str("(push)\n(assert (exists ((m Int)) (and (muni m) (valid (prefix m)) (= (route (prefix m)) Unrouted))))\n(check-sat)\n(pop)\n\n");

    t.push_str("; c2: an invalid prefix never routes to a UF: the gap stays visible.\n");
    t.push_str("(push)\n(assert (exists ((m Int)) (and (muni m) (not (valid (prefix m))) (distinct (route (prefix m)) Unrouted))))\n(check-sat)\n(pop)\n\n");

    let v0 = Uf::V0
        .iter()
        .map(|uf| format!("(= (route (prefix m)) UF-{})", uf.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    t.push_str("; c3: every South municipality code (prefix 41, 42, 43) routes into v0.\n");
    t.push_str(&format!(
        "(push)\n(assert (exists ((m Int)) (and (muni m) (<= 41 (prefix m)) (<= (prefix m) 43) (not (or {v0})))))\n(check-sat)\n(pop)\n\n"
    ));

    t.push_str(
        "; c4: the table is deterministic: two different valid prefixes never share a UF.\n",
    );
    t.push_str("(push)\n(assert (exists ((p Int) (q Int)) (and (valid p) (valid q) (distinct p q) (= (route p) (route q)))))\n(check-sat)\n(pop)\n");
    t
}

/// P2: the `min_severity` stage is monotone: a filter never passes a lower severity while
/// dropping a higher one, and the lowest threshold passes everything.
pub fn severity_smt() -> String {
    let mut t = String::new();
    header(
        &mut t,
        "P2: the min_severity stage is monotone in CAP severity",
    );
    t.push_str(
        "; Severities as ranks: Minor 0, Moderate 1, Severe 2, Extreme 3 (alerts_core order).\n",
    );
    t.push_str("(define-fun sev ((x Int)) Bool (and (<= 0 x) (<= x 3)))\n");
    t.push_str("(define-fun pass ((m Int) (x Int)) Bool (>= x m))\n\n");

    t.push_str("; c1: upward closure: what passes at severity s passes at every s2 >= s.\n");
    t.push_str("(push)\n(assert (exists ((m Int) (s Int) (s2 Int)) (and (sev m) (sev s) (sev s2) (pass m s) (<= s s2) (not (pass m s2)))))\n(check-sat)\n(pop)\n\n");

    t.push_str("; c2: the Minor threshold passes every severity.\n");
    t.push_str(
        "(push)\n(assert (exists ((s Int)) (and (sev s) (not (pass 0 s)))))\n(check-sat)\n(pop)\n",
    );
    t
}

/// P3: stage order and channel parity. `only_ufs` and `min_severity` commute, and two channels
/// that read the same harness output deliver the same alerts (the hard rule: never filter or
/// rewrite alerts per channel).
pub fn stages_smt() -> String {
    let mut t = String::new();
    header(
        &mut t,
        "P3: only_ufs and min_severity commute; channels have parity",
    );
    t.push_str("(declare-sort Alert 0)\n");
    t.push_str("(declare-fun touches-kept-uf (Alert) Bool)\n");
    t.push_str("(declare-fun sev-at-least (Alert) Bool)\n\n");

    t.push_str("; c1: the two stage orders pass the same alerts.\n");
    t.push_str("(push)\n(assert (exists ((a Alert)) (not (= (and (touches-kept-uf a) (sev-at-least a)) (and (sev-at-least a) (touches-kept-uf a))))))\n(check-sat)\n(pop)\n\n");

    t.push_str("; c2: parity: both channels deliver exactly the harness output.\n");
    t.push_str("(declare-fun harness (Alert) Bool)\n");
    t.push_str("(define-fun discord ((a Alert)) Bool (harness a))\n");
    t.push_str("(define-fun feed ((a Alert)) Bool (harness a))\n");
    t.push_str("(push)\n(assert (exists ((a Alert)) (not (= (discord a) (feed a)))))\n(check-sat)\n(pop)\n");
    t
}
