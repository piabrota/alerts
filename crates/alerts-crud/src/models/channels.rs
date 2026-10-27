//! A channel: one delivery target. The record names the environment variable that holds the
//! target, never the secret itself.

use crate::model;
use crate::schema::{Choice, FieldKind, ModuleSpec, Op, field};
use crate::validate::{Checks, FieldError, Validate, is_env_name};

pub const KINDS: &[Choice] = &[
    Choice {
        value: "discord",
        label: "Discord",
    },
    Choice {
        value: "feed",
        label: "Feed Atom + CAP 1.2",
    },
];

model! {
    /// One delivery target of the harness output.
    Channel, ChannelParams in "channels" {
        slug: String,
        kind: String,
        target_env: String,
        enabled: bool,
    }
}

impl Validate for ChannelParams {
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Checks::new()
            .slug("slug", &self.slug)
            .check(
                "kind",
                KINDS.iter().any(|k| k.value == self.kind),
                "Escolha um tipo de canal.",
            )
            .check(
                "target_env",
                is_env_name(&self.target_env),
                "Escreva o nome da variável de ambiente, por exemplo DISCORD_WEBHOOK_URL. Nunca escreva o segredo.",
            )
            .finish()
    }
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "channels",
    label: "Canais",
    singular: "Canal",
    about: "Um canal entrega a saída do harness. Todos os canais de um app recebem os mesmos alertas.",
    fields: &[
        field(
            "slug",
            "Identificador",
            FieldKind::Text,
            "Por exemplo discord.",
            true,
        ),
        field(
            "kind",
            "Tipo",
            FieldKind::Choice { options: KINDS },
            "",
            true,
        ),
        field(
            "target_env",
            "Variável de ambiente",
            FieldKind::Text,
            "O nome da variável que guarda o destino. O segredo fica fora do banco.",
            true,
        ),
        field("enabled", "Ativo", FieldKind::Bool, "", true),
    ],
    ops: Op::CRUD,
    actions: &[],
};
