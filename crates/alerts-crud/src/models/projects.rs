//! A project (the P of PARA): work with an end, inside one area.

use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate};

model! {
    /// One project of an area. The slug is unique inside the area.
    Project, ProjectParams in "projects" {
        area_id: i64,
        slug: String,
        name: String,
        done: bool,
    }
}

impl Validate for ProjectParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .required("name", &self.name)
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "projects",
    label: "Projetos",
    singular: "Projeto",
    about: "Um projeto é um trabalho com fim, dentro de uma área. Ele tem tarefas.",
    fields: &[
        field("area_id", "Área", FieldKind::Int, "O id da área.", true),
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Por exemplo cheia-2026.",
            true,
        ),
        field("name", "Nome", FieldKind::Text, "", true),
        field("done", "Concluído", FieldKind::Bool, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
