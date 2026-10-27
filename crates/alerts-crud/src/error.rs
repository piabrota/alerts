//! The one error type of every controller. It serializes with a `kind` tag, so the interface
//! can show field errors next to their inputs.

use crate::validate::FieldError;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Error {
    /// No record with this id.
    NotFound {
        resource: String,
        id: i64,
    },
    /// The params fail validation. Each entry names a field.
    Invalid {
        errors: Vec<FieldError>,
    },
    /// No resource with this name is registered.
    UnknownResource {
        resource: String,
    },
    /// The resource does not allow this operation (for example, creating an alert).
    NotAllowed {
        resource: String,
        op: String,
    },
    /// The record has no action with this id.
    UnknownAction {
        resource: String,
        action: String,
    },
    Db {
        message: String,
    },
    Json {
        message: String,
    },
}

impl Error {
    pub fn invalid(errors: Vec<FieldError>) -> Self {
        Error::Invalid { errors }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound { resource, id } => write!(f, "{resource} {id}: não existe"),
            Error::Invalid { errors } => {
                let fields: Vec<_> = errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                write!(f, "dados inválidos ({})", fields.join("; "))
            }
            Error::UnknownResource { resource } => write!(f, "recurso desconhecido: {resource}"),
            Error::NotAllowed { resource, op } => {
                write!(f, "{resource}: a operação {op} não é permitida")
            }
            Error::UnknownAction { resource, action } => {
                write!(f, "{resource}: ação desconhecida: {action}")
            }
            Error::Db { message } => write!(f, "banco de dados: {message}"),
            Error::Json { message } => write!(f, "JSON: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db {
            message: e.to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
