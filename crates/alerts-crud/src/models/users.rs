//! A user: the owner of one PARA tree (areas, projects, tasks). There is no login: the record
//! only groups the tree, as Loco would without its accounts.

use crate::model;
use crate::schema::{ActionSpec, FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate};

model! {
    /// One person who composes feeds.
    User, UserParams in "users" {
        slug: String,
        name: String,
    }
}

impl Validate for UserParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .required("name", &self.name)
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "users",
    label: "Usuários",
    singular: "Usuário",
    about: "Um usuário tem as suas áreas, projetos e tarefas, e cada um deles tem feeds e regras.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Por exemplo hoffmann.",
            true,
        ),
        field("name", "Nome", FieldKind::Text, "", true),
    ],
    ops: Op::CRUD,
    actions: &[ActionSpec {
        id: "para",
        label: "Ver áreas, projetos e tarefas",
    }],
};
