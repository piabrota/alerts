//! The domain facts, read from `alerts-core` at run time so the proofs cannot drift from the
//! code. `Facts::gather` observes [`Uf::from_ibge`] over every two-digit prefix; a new prefix in
//! the code appears in the facts, the RDF and the SMT without an edit here.

use alerts_core::{Severity, Uf};

/// An official alert source, as `alerts-catalog` names it. A test asserts that each id and
/// agency below matches the catalog, so these lines cannot drift silently.
#[derive(Debug, Clone, Copy)]
pub struct SourceFact {
    pub id: &'static str,
    pub agency: &'static str,
    pub format: &'static str,
    /// How the source names an area. Both v0 sources use IBGE municipality codes: CEMADEN in
    /// its JSON, IDAP in the CAP 1.2 `<geocode>`.
    pub encoding: &'static str,
}

/// The v0 sources the proofs cover.
pub const SOURCES: &[SourceFact] = &[
    SourceFact {
        id: "cemaden",
        agency: "CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais)",
        format: "JSON",
        encoding: "IbgeMunicipality",
    },
    SourceFact {
        id: "defesa-civil-idap",
        agency: "Defesa Civil Nacional (MIDR)",
        format: "Atom + CAP 1.2",
        encoding: "IbgeMunicipality",
    },
];

/// The v0 channels. Both read the same harness output: parity is by construction, and
/// `p3-stages.smt2` states it.
pub const CHANNELS: &[&str] = &["discord", "feed"];

/// CAP severities, lowest to highest. The guard below fails to compile when `alerts-core` gains
/// a variant, so this list cannot fall behind.
pub const SEVERITIES: [Severity; 4] = [
    Severity::Minor,
    Severity::Moderate,
    Severity::Severe,
    Severity::Extreme,
];

#[allow(dead_code)]
fn every_severity_is_listed(s: Severity) -> usize {
    // Exhaustive match: adding a variant to `Severity` breaks this build until SEVERITIES grows.
    match s {
        Severity::Minor => 0,
        Severity::Moderate => 1,
        Severity::Severe => 2,
        Severity::Extreme => 3,
    }
}

/// The facts the RDF and the SMT are written from.
#[derive(Debug)]
pub struct Facts {
    /// Every two-digit IBGE prefix that routes, with its UF, observed from `Uf::from_ibge`.
    pub prefixes: Vec<(u8, Uf)>,
}

impl Facts {
    pub fn gather() -> Facts {
        let prefixes = (0u8..=99)
            .filter_map(|p| Uf::from_ibge(&format!("{p:02}")).map(|uf| (p, uf)))
            .collect();
        Facts { prefixes }
    }

    /// The rank of a severity, from the order of [`SEVERITIES`].
    pub fn rank(s: Severity) -> usize {
        every_severity_is_listed(s)
    }
}
