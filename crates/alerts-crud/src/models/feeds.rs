//! A feed: one RSS or Atom address that an area, a project or a task reads. The fetcher
//! (`alerts rss-fetch`) stores its items with [`record_items`](crate::models::feed_items::record_items).

use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, SCOPES, field};
use crate::validate::{Checks, FieldError, Validate, is_https_url};

model! {
    /// One RSS or Atom feed. `scope` and `scope_id` name its owner.
    Feed, FeedParams in "feeds" {
        slug: String,
        title: String,
        url: String,
        scope: String,
        scope_id: i64,
        enabled: bool,
    }
}

impl Validate for FeedParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .check(
                "slug",
                self.slug != alerts_rss::ALERTS,
                "O identificador alerts é dos alertas oficiais.",
            )
            .required("title", &self.title)
            .check(
                "url",
                is_https_url(&self.url),
                "Use o endereço HTTPS do feed RSS ou Atom.",
            )
            .check(
                "scope",
                SCOPES.iter().any(|s| s.value == self.scope),
                "Escolha área, projeto ou tarefa.",
            )
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "feeds",
    label: "Feeds RSS",
    singular: "Feed RSS",
    about: "Um feed RSS ou Atom que uma área, um projeto ou uma tarefa lê. As regras combinam os feeds.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "As regras usam este nome, por exemplo g1-sc.",
            true,
        ),
        field("title", "Título", FieldKind::Text, "", true),
        field(
            "url",
            "Endereço",
            FieldKind::Url,
            "O endereço HTTPS do feed.",
            false,
        ),
        field(
            "scope",
            "Dono",
            FieldKind::Choice { options: SCOPES },
            "",
            true,
        ),
        field(
            "scope_id",
            "Id do dono",
            FieldKind::Int,
            "O id da área, do projeto ou da tarefa.",
            true,
        ),
        field("enabled", "Ativo", FieldKind::Bool, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
