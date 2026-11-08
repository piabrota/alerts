//! The same invariants, checked exhaustively in Rust against the real `alerts-core` code. The
//! domains are finite, so these checks are proofs in practice; the SMT files restate them for an
//! external prover (`just prove`). A violation names its counterexample.

use crate::facts::{Facts, SEVERITIES};
use alerts_core::{min_severity, only_ufs, Alert, Harness, Hazard, Severity, Uf};

fn alert(uf: Uf, severity: Severity) -> Alert {
    Alert {
        id: "proofs-1".into(),
        source: "proofs".into(),
        hazard: Hazard::Flood,
        severity,
        ufs: vec![uf],
        headline: "Teste.".into(),
        area: "Teste".into(),
        sent: "2026-01-01T00:00:00Z".into(),
        url: None,
    }
}

/// Runs every check; the first violation is the error.
pub fn run(facts: &Facts) -> Result<(), String> {
    // C1: the two-digit table and the seven-digit form agree, over every prefix.
    for p in 0u8..=99 {
        let two = Uf::from_ibge(&format!("{p:02}"));
        for k in [0u32, 12345, 99999] {
            let seven = Uf::from_ibge(&format!("{p:02}{k:05}"));
            if two != seven {
                return Err(format!(
                    "C1: from_ibge disagrees between {p:02} and {p:02}{k:05}: {two:?} vs {seven:?}"
                ));
            }
        }
    }

    // C1b: malformed codes never route; surrounding whitespace is trimmed by contract.
    for bad in ["", "4", "431490", "43a4902", "431490200", "43 4902"] {
        if Uf::from_ibge(bad).is_some() {
            return Err(format!("C1b: from_ibge routed the malformed code {bad:?}"));
        }
    }
    if Uf::from_ibge(" 43 ") != Some(Uf::Rs) {
        return Err("C1b: from_ibge no longer trims surrounding whitespace".into());
    }

    // C2: the South prefixes land exactly on Uf::V0.
    let south: Vec<Uf> = facts
        .prefixes
        .iter()
        .filter(|(p, _)| (41..=43).contains(p))
        .map(|&(_, uf)| uf)
        .collect();
    if south != Uf::V0 {
        return Err(format!("C2: prefixes 41..=43 route to {south:?}, not Uf::V0"));
    }

    // C2b: the observed table is deterministic: no UF is reached by two prefixes.
    for (i, &(p, uf)) in facts.prefixes.iter().enumerate() {
        for &(q, vf) in &facts.prefixes[i + 1..] {
            if uf == vf {
                return Err(format!("C2b: prefixes {p} and {q} both route to {uf}"));
            }
        }
    }

    // C3: the severity ranks follow the alerts-core order, strictly.
    for (i, &a) in SEVERITIES.iter().enumerate() {
        for &b in &SEVERITIES[i + 1..] {
            if a >= b {
                return Err(format!("C3: severity order broken: {a:?} >= {b:?}"));
            }
        }
        if Facts::rank(a) != i {
            return Err(format!("C3: rank of {a:?} is {}, not {i}", Facts::rank(a)));
        }
    }

    // C3b: the real min_severity stage is upward closed, over every (threshold, severity).
    for &min in &SEVERITIES {
        let stage = min_severity(min);
        for &s in &SEVERITIES {
            let passed = stage(alert(Uf::Rs, s)).is_some();
            if passed != (s >= min) {
                return Err(format!("C3b: min_severity({min:?}) on {s:?}: passed={passed}"));
            }
            for &s2 in &SEVERITIES {
                if passed && s2 >= s && stage(alert(Uf::Rs, s2)).is_none() {
                    return Err(format!(
                        "C3b: min_severity({min:?}) passes {s:?} and drops {s2:?}"
                    ));
                }
            }
        }
    }

    // C4: only_ufs and min_severity commute, over every (UF, severity), in the real Harness.
    for &uf in Uf::ALL {
        for &s in &SEVERITIES {
            let ab = Harness::new()
                .then(only_ufs(Uf::V0.to_vec()))
                .then(min_severity(Severity::Moderate))
                .run(alert(uf, s));
            let ba = Harness::new()
                .then(min_severity(Severity::Moderate))
                .then(only_ufs(Uf::V0.to_vec()))
                .run(alert(uf, s));
            if ab != ba {
                return Err(format!("C4: stage order differs on ({uf}, {s:?})"));
            }
        }
    }

    // C5: parity: two channels reading the same harness deliver the same alerts.
    let channel = || {
        Harness::new()
            .then(only_ufs(Uf::V0.to_vec()))
            .then(min_severity(Severity::Minor))
    };
    let (discord, feed) = (channel(), channel());
    for &uf in Uf::ALL {
        for &s in &SEVERITIES {
            if discord.run(alert(uf, s)) != feed.run(alert(uf, s)) {
                return Err(format!("C5: channel parity broken on ({uf}, {s:?})"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::SOURCES;

    #[test]
    fn every_check_holds() {
        let facts = Facts::gather();
        if let Err(e) = run(&facts) {
            panic!("{e}");
        }
    }

    #[test]
    fn source_facts_match_the_catalog() {
        for s in SOURCES {
            let found = alerts_catalog::get(s.id)
                .unwrap_or_else(|| panic!("source {} is not in alerts-catalog", s.id));
            assert_eq!(found.agency, s.agency, "agency of {}", s.id);
            assert_eq!(found.format, s.format, "format of {}", s.id);
        }
    }
}
