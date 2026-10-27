//! A rule: one composed feed. `expr` is the JSON of an [`alerts_rss::Expr`], for example
//! `{"op": "union", "of": [{"op": "feed", "feed": "g1-sc"}, {"op": "alerts"}]}`.

use crate::model;
use crate::schema::{ActionSpec, FieldKind, ModuleSpec, Op, SCOPES, field};
use crate::validate::{Checks, FieldError, Validate};
use alerts_rss::Expr;

model! {
    /// One composition of feeds. `scope` and `scope_id` name its owner.
    Rule, RuleParams in "rules" {
        slug: String,
        title: String,
        expr: String,
        scope: String,
        scope_id: i64,
    }
}

impl RuleParams {
    pub fn parse_expr(&self) -> Result<Expr, serde_json::Error> {
        serde_json::from_str(&self.expr)
    }
}

impl Rule {
    pub fn parse_expr(&self) -> Result<Expr, serde_json::Error> {
        serde_json::from_str(&self.expr)
    }
}

impl Validate for RuleParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let expr = self.parse_expr();
        Checks::new()
            .slug("slug", &self.slug)
            .required("title", &self.title)
            .check(
                "expr",
                expr.is_ok(),
                &expr.err().map_or(String::new(), |e| {
                    format!("A regra não é uma expressão válida: {e}")
                }),
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
    resource: "rules",
    label: "Regras de feed",
    singular: "Regra de feed",
    about: "Uma regra combina feeds, alertas oficiais e outras regras: união, interseção, diferença, filtro, mapa, sem repetição e limite.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Outras regras usam este nome.",
            true,
        ),
        field("title", "Título", FieldKind::Text, "", true),
        field(
            "expr",
            "Expressão",
            FieldKind::LongText,
            "JSON, por exemplo {\"op\": \"union\", \"of\": [{\"op\": \"feed\", \"feed\": \"g1-sc\"}, {\"op\": \"alerts\"}]}.",
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
    ],
    ops: Op::CRUD,
    actions: &[ActionSpec {
        id: "atom",
        label: "Ver feed Atom",
    }],
};
