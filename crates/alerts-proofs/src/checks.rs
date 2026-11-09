//! The same invariants, checked exhaustively in Rust against the real `alerts-core` code. The
//! domains are finite, so these checks are proofs in practice; the SMT files restate them for an
//! external prover (`just prove`). A violation names its counterexample.

use crate::facts::{Facts, SEVERITIES};
use alerts_core::{Alert, Harness, Hazard, Severity, Uf, min_severity, only_ufs};
use alerts_rss::{Env, Expr, Item, Pred, TextField, time::normalize};

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
        return Err(format!(
            "C2: prefixes 41..=43 route to {south:?}, not Uf::V0"
        ));
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
                return Err(format!(
                    "C3b: min_severity({min:?}) on {s:?}: passed={passed}"
                ));
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

    // C7: the hypothesis of P6 `identifier_unique_when_ids_start_with_a_digit`, over the catalog:
    // every source id is a slug with no `-` before a digit.
    for e in alerts_catalog::CATALOG {
        let b = e.id.as_bytes();
        let dash_digit = b.windows(2).any(|w| w[0] == b'-' && w[1].is_ascii_digit());
        if e.id.is_empty() || b[0] == b'-' || b[b.len() - 1] == b'-' || dash_digit {
            return Err(format!("C7: catalog id {:?} breaks the form of P6", e.id));
        }
    }

    witnesses()
}

/// The counterexamples of P4 to P6, replayed through the real code: each `expect: sat` of
/// `code.rs` is a fact about the Rust, not only about its model.
fn witnesses() -> Result<(), String> {
    let w = |name: &str, ok: bool, what: &str| {
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{name}: the counterexample no longer holds: {what}"
            ))
        }
    };
    let utc = |s: &str| normalize(s);

    // W1 (P4 accepted_dates_are_dates): February 31 is accepted and becomes March 3.
    w(
        "W1",
        utc("2026-02-31T00:00:00Z").as_deref() == Some("2026-03-03T00:00:00+00:00"),
        "normalize(2026-02-31) is 2026-03-03",
    )?;
    // W2 (P4 accepted_clock_within_the_day): 23:59:60 is the next midnight.
    w(
        "W2",
        utc("2026-10-22T23:59:60Z") == utc("2026-10-23T00:00:00Z")
            && utc("2026-10-22T23:59:60Z").is_some(),
        "23:59:60 equals 00:00:00 of the next day",
    )?;
    // W3 (P4 offset_within_a_day): +00:60 is +01:00, and +99:99 is accepted.
    w(
        "W3",
        utc("2026-10-22T12:00:00+00:60") == utc("2026-10-22T12:00:00+01:00")
            && utc("2026-10-22T12:00:00+99:99").is_some(),
        "the offset is not range-checked",
    )?;
    // W4 (P4 utc_year_has_four_digits): the year 10000 prints with five digits and sorts first.
    let far = utc("10000-01-01T00:00:00Z");
    let now = utc("2026-10-22T00:00:00Z");
    w(
        "W4",
        far.as_deref() == Some("10000-01-01T00:00:00+00:00") && far < now,
        "the year 10000 sorts before 2026",
    )?;

    // W5 (P5 filter_union_distributes): two feeds give one link (the key) two titles.
    let item = |feed: &str, title: &str| Item {
        feed: feed.into(),
        key: "https://example.org/a".into(),
        title: title.into(),
        link: None,
        summary: String::new(),
        updated: None,
        alert: None,
    };
    let mut env = Env::default();
    env.feeds
        .insert("g1".into(), vec![item("g1", "Enchente no Vale")]);
    env.feeds.insert(
        "uol".into(),
        vec![item("uol", "Enchente no Vale do Itajaí")],
    );
    let feed = |f: &str| Expr::Feed { feed: f.into() };
    let p = Pred::Contains {
        field: TextField::Title,
        text: "itajaí".into(),
    };
    let filter = |of: Expr| Expr::Filter {
        when: p.clone(),
        of: Box::new(of),
    };
    let left = alerts_rss::eval(
        &filter(Expr::Union {
            of: vec![feed("g1"), feed("uol")],
        }),
        &env,
    );
    let right = alerts_rss::eval(
        &Expr::Union {
            of: vec![filter(feed("g1")), filter(feed("uol"))],
        },
        &env,
    );
    w(
        "W5",
        matches!((&left, &right), (Ok(l), Ok(r)) if l.is_empty() && r.len() == 1),
        "filter(p, union(g1, uol)) is empty and union(filter(p, g1), filter(p, uol)) holds the uol item",
    )?;

    // W6 (P6 identifier_unique): two catalog sources, two ids, one CAP identifier.
    let alert = |source: &str, id: &str| Alert {
        id: id.into(),
        source: source.into(),
        hazard: Hazard::Flood,
        severity: Severity::Severe,
        ufs: vec![Uf::Sc],
        headline: "Teste.".into(),
        area: "Teste".into(),
        sent: "2026-01-01T00:00:00Z".into(),
        url: None,
    };
    let known =
        alerts_catalog::get("cemaden").is_some() && alerts_catalog::get("cemaden-ped").is_some();
    w(
        "W6",
        known
            && alerts_feed::identifier(&alert("cemaden", "ped-1"))
                == alerts_feed::identifier(&alert("cemaden-ped", "1")),
        "cemaden/ped-1 and cemaden-ped/1 share an identifier",
    )?;

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
