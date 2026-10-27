//! Validation of params, in the spirit of Loco's `Validatable`: each check returns the errors of
//! one field, and a model joins them. The messages are pt-BR, because the interface shows them.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

pub trait Validate {
    /// Every error of the params, or `Ok` when there is none.
    fn validate(&self) -> Result<(), Vec<FieldError>>;
}

/// Collects field errors. `finish` returns `Ok` only when no check failed.
#[derive(Debug, Default)]
pub struct Checks(Vec<FieldError>);

impl Checks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(mut self, field: &str, ok: bool, message: &str) -> Self {
        if !ok {
            self.0.push(FieldError {
                field: field.to_owned(),
                message: message.to_owned(),
            });
        }
        self
    }

    pub fn required(self, field: &str, value: &str) -> Self {
        self.check(field, !value.trim().is_empty(), "O campo é obrigatório.")
    }

    /// Lowercase ASCII letters, digits and `-`, as in `defesa-civil-idap`.
    pub fn slug(self, field: &str, value: &str) -> Self {
        self.check(
            field,
            is_slug(value),
            "Use letras minúsculas, números e hífen, por exemplo defesa-civil-idap.",
        )
    }

    pub fn non_empty<T>(self, field: &str, values: &[T]) -> Self {
        self.check(field, !values.is_empty(), "Escolha pelo menos uma opção.")
    }

    pub fn finish(self) -> Result<(), Vec<FieldError>> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(self.0)
        }
    }
}

pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `every <N><unit>`, with unit `s`, `m` or `h`: the poll rate form of `app.toml`.
pub fn is_every(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("every ") else {
        return false;
    };
    let (n, unit) = rest.split_at(rest.len().saturating_sub(1));
    matches!(unit, "s" | "m" | "h") && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
}

/// An environment variable name: uppercase ASCII letters, digits and `_`, not starting with a
/// digit. A channel names the variable that holds its secret, never the secret.
pub fn is_env_name(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(|c: char| c.is_ascii_digit())
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn is_https_url(s: &str) -> bool {
    s.strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn slugs() {
        assert!(is_slug("defesa-civil-idap"));
        assert!(is_slug("inmet"));
        assert!(!is_slug("Inmet"));
        assert!(!is_slug("-inmet"));
        assert!(!is_slug("in met"));
        assert!(!is_slug(""));
    }

    #[test]
    fn poll_rates() {
        assert!(is_every("every 5m"));
        assert!(is_every("every 30s"));
        assert!(!is_every("every m"));
        assert!(!is_every("5m"));
        assert!(!is_every("every 5d"));
    }

    #[test]
    fn env_names() {
        assert!(is_env_name("DISCORD_WEBHOOK_URL"));
        assert!(!is_env_name("https://discord.com/api/webhooks/1"));
        assert!(!is_env_name("1PATH"));
    }

    #[test]
    fn checks_collect_every_error() {
        let r = Checks::new()
            .required("name", "")
            .slug("slug", "A B")
            .finish();
        let fields: Vec<_> = r
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.field)
            .collect();
        assert_eq!(fields, ["name", "slug"]);
    }
}
