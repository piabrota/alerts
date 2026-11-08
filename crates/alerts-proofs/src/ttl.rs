//! The facts as RDF 1.1 Turtle: the vocabulary a SPARQL store, the wiki or a future SHACL shape
//! reads. One file, stable order, written only from [`Facts`].

use crate::facts::{Facts, CHANNELS, SEVERITIES, SOURCES};
use alerts_core::Uf;

pub fn facts_ttl(facts: &Facts) -> String {
    let mut t = String::new();
    t.push_str("# The alert domain of piabrota/alerts as RDF, written by `just proofs`\n");
    t.push_str("# (crates/alerts-proofs). Do not edit: `just proofs-check` compares this file\n");
    t.push_str("# with the code.\n");
    t.push_str("@prefix al: <https://github.com/piabrota/alerts/ns#> .\n");
    t.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n");

    for (p, uf) in &facts.prefixes {
        t.push_str(&format!(
            "al:br-uf-{} a al:Uf ;\n    al:ibgePrefix \"{p:02}\"^^xsd:integer .\n",
            uf.as_str()
        ));
    }
    t.push('\n');

    for s in SEVERITIES {
        t.push_str(&format!(
            "al:severity-{} a al:Severity ;\n    al:rank {} ;\n    al:labelPtBr \"{}\" .\n",
            s.label_pt_br(),
            Facts::rank(s),
            s.label_pt_br()
        ));
    }
    t.push('\n');

    let v0 = Uf::V0
        .iter()
        .map(|uf| format!("al:br-uf-{}", uf.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    t.push_str(&format!("al:v0 a al:Scope ;\n    al:serves {v0} .\n\n"));

    for s in SOURCES {
        t.push_str(&format!(
            "al:source-{} a al:Source ;\n    al:agency \"{}\" ;\n    al:format \"{}\" ;\n    al:areaEncoding al:{} ;\n    al:official true .\n",
            s.id, s.agency, s.format, s.encoding
        ));
    }
    t.push('\n');

    for c in CHANNELS {
        t.push_str(&format!(
            "al:channel-{c} a al:Channel ;\n    al:readsHarness al:harness .\n"
        ));
    }
    t
}
