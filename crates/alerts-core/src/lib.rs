//! Domain types and the FP harness of alerts.
//!
//! A harness is a composition of pure stages. Each stage takes an [`Alert`] and returns
//! `Some(alert)` to pass it on, possibly changed, or `None` to drop it. Specializing a
//! harness means composing one more stage, never overriding a method.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

macro_rules! ufs {
    ($($v:ident = $s:literal),* $(,)?) => {
        /// A Brazilian federative unit (state or Distrito Federal).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "UPPERCASE")]
        pub enum Uf { $($v),* }

        /// The User-Agent of every alerts HTTP request: an honest name and a way to reach us.
pub const USER_AGENT: &str = concat!(
    "piabrota-alerts/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/piabrota/alerts)"
);

impl Uf {
            pub const ALL: &[Uf] = &[$(Uf::$v),*];

            pub fn as_str(self) -> &'static str {
                match self { $(Uf::$v => $s),* }
            }
        }

        impl FromStr for Uf {
            type Err = UnknownUf;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim().to_ascii_uppercase().as_str() {
                    $($s => Ok(Uf::$v),)*
                    _ => Err(UnknownUf(s.to_owned())),
                }
            }
        }
    };
}

ufs! {
    Ac = "AC", Al = "AL", Ap = "AP", Am = "AM", Ba = "BA", Ce = "CE", Df = "DF",
    Es = "ES", Go = "GO", Ma = "MA", Mt = "MT", Ms = "MS", Mg = "MG", Pa = "PA",
    Pb = "PB", Pr = "PR", Pe = "PE", Pi = "PI", Rj = "RJ", Rn = "RN", Rs = "RS",
    Ro = "RO", Rr = "RR", Sc = "SC", Sp = "SP", Se = "SE", To = "TO",
}

impl Uf {
    /// The states v0 serves: the South region.
    pub const V0: &[Uf] = &[Uf::Pr, Uf::Sc, Uf::Rs];
}

impl fmt::Display for Uf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownUf(pub String);

impl fmt::Display for UnknownUf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown UF: {:?}", self.0)
    }
}

impl std::error::Error for UnknownUf {}

/// CAP severity, ordered from lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Minor,
    Moderate,
    Severe,
    Extreme,
}

impl Severity {
    /// The pt-BR label every channel shows, so all channels say the same thing.
    pub fn label_pt_br(self) -> &'static str {
        match self {
            Severity::Minor => "menor",
            Severity::Moderate => "moderada",
            Severity::Severe => "severa",
            Severity::Extreme => "extrema",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hazard {
    Flood,
}

impl Hazard {
    pub fn label_pt_br(self) -> &'static str {
        match self {
            Hazard::Flood => "Inundação",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    /// Stable id given by the source, used for deduplication.
    pub id: String,
    /// Source module id, for example `inmet`.
    pub source: String,
    pub hazard: Hazard,
    pub severity: Severity,
    pub ufs: Vec<Uf>,
    /// Controlled pt-BR text, one statement per sentence.
    pub headline: String,
    pub area: String,
    /// ISO 8601 time the source sent the alert.
    pub sent: String,
    pub url: Option<String>,
}

pub type Stage = Box<dyn Fn(Alert) -> Option<Alert> + Send + Sync>;

/// An ordered composition of stages. The empty harness passes every alert.
#[derive(Default)]
pub struct Harness {
    stages: Vec<Stage>,
}

impl fmt::Debug for Harness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Harness")
            .field("stages", &self.stages.len())
            .finish()
    }
}

impl Harness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn then(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn run(&self, alert: Alert) -> Option<Alert> {
        self.stages.iter().try_fold(alert, |a, stage| stage(a))
    }
}

/// Keeps alerts that touch at least one of `ufs`.
pub fn only_ufs(ufs: Vec<Uf>) -> Stage {
    Box::new(move |a| a.ufs.iter().any(|u| ufs.contains(u)).then_some(a))
}

/// Keeps alerts at `min` severity or above.
pub fn min_severity(min: Severity) -> Stage {
    Box::new(move |a| (a.severity >= min).then_some(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(ufs: Vec<Uf>, severity: Severity) -> Alert {
        Alert {
            id: "t-1".into(),
            source: "test".into(),
            hazard: Hazard::Flood,
            severity,
            ufs,
            headline: "Teste.".into(),
            area: "Porto Alegre".into(),
            sent: "2026-10-22T12:00:00-03:00".into(),
            url: None,
        }
    }

    #[test]
    fn uf_round_trip() {
        for &uf in Uf::ALL {
            assert_eq!(uf.as_str().parse::<Uf>(), Ok(uf));
        }
        assert_eq!(" rs ".parse::<Uf>(), Ok(Uf::Rs));
        assert!("XX".parse::<Uf>().is_err());
        assert_eq!(Uf::ALL.len(), 27);
    }

    #[test]
    fn harness_composes_stages() {
        let h = Harness::new()
            .then(only_ufs(Uf::V0.to_vec()))
            .then(min_severity(Severity::Moderate));
        assert!(h.run(alert(vec![Uf::Rs], Severity::Severe)).is_some());
        assert!(h.run(alert(vec![Uf::Sp], Severity::Severe)).is_none());
        assert!(h.run(alert(vec![Uf::Sc], Severity::Minor)).is_none());
        assert!(Harness::new().run(alert(vec![], Severity::Minor)).is_some());
    }
}
