//! A source: one agency feed that a source crate (`crates/alerts-source-<id>`) reads.

use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate, is_every, is_https_url};

model! {
    /// One agency feed. `slug` is the id of its source crate.
    Source, SourceParams in "sources" {
        slug: String,
        name: String,
        agency: String,
        url: String,
        poll_every: String,
        enabled: bool,
    }
}

impl Validate for SourceParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .required("name", &self.name)
            .required("agency", &self.agency)
            .check(
                "url",
                is_https_url(&self.url),
                "Use o endereço HTTPS da fonte oficial.",
            )
            .check(
                "poll_every",
                is_every(&self.poll_every),
                "Use a forma every <N><s|m|h>, por exemplo every 5m.",
            )
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "sources",
    label: "Fontes",
    singular: "Fonte",
    about: "Uma fonte é o feed de um órgão oficial. Cada fonte tem um crate alerts-source-<id>.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "O id do crate, por exemplo inmet.",
            true,
        ),
        field("name", "Nome", FieldKind::Text, "", true),
        field(
            "agency",
            "Órgão",
            FieldKind::Text,
            "O órgão oficial que publica o alerta.",
            true,
        ),
        field(
            "url",
            "Endereço",
            FieldKind::Url,
            "O endereço HTTPS do feed.",
            false,
        ),
        field(
            "poll_every",
            "Frequência de leitura",
            FieldKind::Text,
            "Por exemplo every 5m. Respeite o limite da fonte.",
            true,
        ),
        field("enabled", "Ativa", FieldKind::Bool, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
