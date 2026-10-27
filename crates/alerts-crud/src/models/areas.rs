//! An area (the A of PARA): a part of life with no end date, for example saúde or a city.

use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate};

model! {
    /// One area of a user. The slug is unique inside the user.
    Area, AreaParams in "areas" {
        user_id: i64,
        slug: String,
        name: String,
    }
}

impl Validate for AreaParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .required("name", &self.name)
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "areas",
    label: "Áreas",
    singular: "Área",
    about: "Uma área é uma responsabilidade sem data de fim. Ela tem projetos.",
    fields: &[
        field(
            "user_id",
            "Usuário",
            FieldKind::Int,
            "O id do usuário.",
            true,
        ),
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Por exemplo enchentes-sc.",
            true,
        ),
        field("name", "Nome", FieldKind::Text, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
