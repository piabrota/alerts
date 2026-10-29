//! BYOT (bring your own token): the tokens of the catalog sources that need one.
//!
//! alerts never ships a token. Each person registers with the agency and sets the token in the
//! variable that the catalog names (`ALERTS_TOKEN_<ID>`), in the environment or in `.env`. A token
//! is a secret like a webhook URL: it never reaches a message, a log or an error.

use alerts_catalog::{Auth, Entry};
use std::fmt;

/// A token. `Debug` and `Display` never show it; only [`Token::expose`] gives the value, to put
/// it in a request.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<hidden>)")
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<hidden>")
    }
}

/// The token state of one catalog entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenState {
    /// The endpoint needs no token.
    NotNeeded,
    Set(Token),
    Missing {
        env: &'static str,
    },
}

impl TokenState {
    /// The state as one short line for people, never with the token.
    pub fn describe(&self) -> String {
        match self {
            TokenState::NotNeeded => "no token".into(),
            TokenState::Set(_) => "token set".into(),
            TokenState::Missing { env } => format!("token missing: set {env}"),
        }
    }
}

/// The token of `entry`, read by `lookup` (the environment, in the service).
pub fn state(entry: &Entry, lookup: impl Fn(&str) -> Option<String>) -> TokenState {
    match entry.auth {
        Auth::None => TokenState::NotNeeded,
        Auth::Token { env, .. } => match lookup(env).map(|v| v.trim().to_owned()) {
            Some(v) if !v.is_empty() => TokenState::Set(Token(v)),
            _ => TokenState::Missing { env },
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const SECRET: &str = "eyJhbGciOi.secret";

    fn entry(id: &str) -> &'static Entry {
        alerts_catalog::get(id).unwrap()
    }

    #[test]
    fn a_token_is_never_shown() {
        let state = state(entry("cemaden-ped"), |_| Some(SECRET.into()));
        let TokenState::Set(t) = &state else {
            panic!("{state:?}")
        };
        assert_eq!(t.expose(), SECRET);
        for shown in [
            format!("{t}"),
            format!("{t:?}"),
            format!("{state:?}"),
            state.describe(),
        ] {
            assert!(!shown.contains("secret"), "{shown}");
        }
    }

    #[test]
    fn missing_and_empty_tokens_name_the_variable() {
        let e = entry("meteo-france");
        for lookup in [None, Some(String::new()), Some("  ".into())] {
            let s = state(e, move |_| lookup.clone());
            assert_eq!(
                s,
                TokenState::Missing {
                    env: "ALERTS_TOKEN_METEO_FRANCE"
                }
            );
            assert_eq!(s.describe(), "token missing: set ALERTS_TOKEN_METEO_FRANCE");
        }
    }

    #[test]
    fn free_sources_need_no_token() {
        assert_eq!(state(entry("cemaden"), |_| None), TokenState::NotNeeded);
    }

    #[test]
    fn the_variable_is_the_one_of_the_entry() {
        let seen = std::cell::RefCell::new(Vec::new());
        let _ = state(entry("cemaden-ped"), |name| {
            seen.borrow_mut().push(name.to_owned());
            None
        });
        assert_eq!(*seen.borrow(), ["ALERTS_TOKEN_CEMADEN_PED"]);
    }
}
