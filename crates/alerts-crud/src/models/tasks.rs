//! A task: one step of a project.

use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate};

model! {
    /// One task of a project.
    Task, TaskParams in "tasks" {
        project_id: i64,
        title: String,
        done: bool,
    }
}

impl Validate for TaskParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new().required("title", &self.title).finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "tasks",
    label: "Tarefas",
    singular: "Tarefa",
    about: "Uma tarefa é um passo de um projeto.",
    fields: &[
        field(
            "project_id",
            "Projeto",
            FieldKind::Int,
            "O id do projeto.",
            true,
        ),
        field("title", "Título", FieldKind::Text, "", true),
        field("done", "Concluída", FieldKind::Bool, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
