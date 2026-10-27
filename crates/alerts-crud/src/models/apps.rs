//! An app: one deployment for one audience. It joins sources, a harness and channels, and it
//! exports to the `apps/<app>/app.toml` that the `manage-apps` skill describes.

use crate::model;
use crate::models::sources::Source;
use crate::schema::{ActionSpec, FieldKind, HAZARDS, ModuleSpec, Op, SEVERITIES, UFS, field};
use crate::validate::{Checks, FieldError, Validate};
use alerts_core::{Harness, Hazard, Severity, Uf, min_severity, only_ufs};
use serde::Serialize;
use std::collections::BTreeMap;

model! {
    /// One deployment. `sources` and `channels` hold slugs of those resources.
    App, AppParams in "apps" {
        slug: String,
        name: String,
        hazards: Vec<Hazard>,
        ufs: Vec<Uf>,
        sources: Vec<String>,
        min_severity: Severity,
        channels: Vec<String>,
    }
}

impl Validate for AppParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .required("name", &self.name)
            .non_empty("hazards", &self.hazards)
            .non_empty("ufs", &self.ufs)
            .non_empty("sources", &self.sources)
            .non_empty("channels", &self.channels)
            .finish()
    }
}

impl App {
    /// The harness of this app: the stages that `app.toml` names, in the same order.
    pub fn harness(&self) -> Harness {
        Harness::new()
            .then(only_ufs(self.ufs.clone()))
            .then(min_severity(self.min_severity))
    }

    fn stage_names(&self) -> Vec<String> {
        vec![
            "only_ufs".into(),
            format!("min_severity:{}", lower(&self.min_severity)),
        ]
    }

    /// The `app.toml` of this app. `sources` gives the poll rate of each source it names.
    pub fn to_toml(&self, sources: &[Source]) -> Result<String, toml::ser::Error> {
        #[derive(Serialize)]
        struct AppToml<'a> {
            name: &'a str,
            hazards: Vec<String>,
            ufs: &'a [Uf],
            sources: &'a [String],
            stages: Vec<String>,
            channels: &'a [String],
            schedule: BTreeMap<&'a str, &'a str>,
        }
        let schedule = sources
            .iter()
            .filter(|s| self.sources.contains(&s.slug))
            .map(|s| (s.slug.as_str(), s.poll_every.as_str()))
            .collect();
        let body = toml::to_string(&AppToml {
            name: &self.slug,
            hazards: self.hazards.iter().map(lower).collect(),
            ufs: &self.ufs,
            sources: &self.sources,
            stages: self.stage_names(),
            channels: &self.channels,
            schedule,
        })?;
        Ok(format!(
            "# apps/{}/app.toml: {}. The secrets come from the environment.\n{body}",
            self.slug, self.name
        ))
    }
}

/// The serde name of a unit enum, lowercase: `Severity::Moderate` is `moderate`.
fn lower<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s.to_lowercase(),
        _ => String::new(),
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "apps",
    label: "Apps",
    singular: "App",
    about: "Um app é uma implantação para um público: fontes, harness e canais.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Por exemplo sul-enchentes.",
            true,
        ),
        field("name", "Nome", FieldKind::Text, "", true),
        field(
            "hazards",
            "Ameaças",
            FieldKind::Many { options: HAZARDS },
            "",
            true,
        ),
        field(
            "ufs",
            "UFs",
            FieldKind::Many { options: UFS },
            "Só marque uma UF que uma fonte cobre.",
            true,
        ),
        field(
            "sources",
            "Fontes",
            FieldKind::Refs {
                resource: "sources",
            },
            "",
            true,
        ),
        field(
            "min_severity",
            "Severidade mínima",
            FieldKind::Choice {
                options: SEVERITIES,
            },
            "O harness descarta os alertas abaixo desta severidade.",
            true,
        ),
        field(
            "channels",
            "Canais",
            FieldKind::Refs {
                resource: "channels",
            },
            "",
            true,
        ),
    ],
    ops: Op::CRUD,
    actions: &[ActionSpec {
        id: "app_toml",
        label: "Ver app.toml",
    }],
};
