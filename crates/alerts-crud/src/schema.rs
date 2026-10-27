//! What the interface needs to draw a resource: its labels, its fields and its operations. The
//! interface has no code per resource. It draws the table and the form from a [`ModuleSpec`],
//! so a scaffolded resource shows up without a change in the interface.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    LongText,
    Url,
    Bool,
    Int,
    /// One of `options`.
    Choice {
        options: &'static [Choice],
    },
    /// Any number of `options`, stored as a list.
    Many {
        options: &'static [Choice],
    },
    /// Any number of records of another resource, stored as their slugs.
    Refs {
        resource: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FieldSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    /// A short pt-BR hint under the input.
    pub help: &'static str,
    /// Show this field as a column of the table.
    pub list: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    List,
    Show,
    Create,
    Update,
    Delete,
}

impl Op {
    pub const CRUD: &[Op] = &[Op::List, Op::Show, Op::Create, Op::Update, Op::Delete];
    pub const READ_DELETE: &[Op] = &[Op::List, Op::Show, Op::Delete];

    pub fn as_str(self) -> &'static str {
        match self {
            Op::List => "list",
            Op::Show => "show",
            Op::Create => "create",
            Op::Update => "update",
            Op::Delete => "delete",
        }
    }
}

/// An extra operation on one record that returns text, for example the `app.toml` of an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ActionSpec {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModuleSpec {
    /// The resource name, which is also the table name.
    pub resource: &'static str,
    /// The pt-BR name in the menu, plural.
    pub label: &'static str,
    /// The pt-BR name of one record.
    pub singular: &'static str,
    /// One pt-BR sentence at the top of the page.
    pub about: &'static str,
    pub fields: &'static [FieldSpec],
    pub ops: &'static [Op],
    pub actions: &'static [ActionSpec],
}

pub const fn field(
    name: &'static str,
    label: &'static str,
    kind: FieldKind,
    help: &'static str,
    list: bool,
) -> FieldSpec {
    FieldSpec {
        name,
        label,
        kind,
        help,
        list,
    }
}

pub const SEVERITIES: &[Choice] = &[
    Choice {
        value: "Minor",
        label: "menor",
    },
    Choice {
        value: "Moderate",
        label: "moderada",
    },
    Choice {
        value: "Severe",
        label: "severa",
    },
    Choice {
        value: "Extreme",
        label: "extrema",
    },
];

pub const HAZARDS: &[Choice] = &[Choice {
    value: "Flood",
    label: "Inundação",
}];

macro_rules! uf_choices {
    ($($uf:literal),*) => { &[$(Choice { value: $uf, label: $uf }),*] };
}

/// Every UF, the v0 states first.
pub const UFS: &[Choice] = uf_choices!(
    "PR", "SC", "RS", "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG",
    "PA", "PB", "PE", "PI", "RJ", "RN", "RO", "RR", "SP", "SE", "TO"
);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use alerts_core::{Hazard, Severity, Uf};

    #[test]
    fn choices_match_the_core_types() {
        for c in SEVERITIES {
            let s: Severity = serde_json::from_value(c.value.into()).unwrap();
            assert_eq!(s.label_pt_br(), c.label);
        }
        for c in HAZARDS {
            let h: Hazard = serde_json::from_value(c.value.into()).unwrap();
            assert_eq!(h.label_pt_br(), c.label);
        }
        assert_eq!(UFS.len(), Uf::ALL.len());
        for c in UFS {
            assert!(c.value.parse::<Uf>().is_ok());
        }
    }
}
