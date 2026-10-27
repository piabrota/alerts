//! The rule algebra. An [`Expr`] is a value: it serializes to JSON, the interface stores it,
//! and [`eval`] reads it. Every operation works on item *sets*: the identity of an item is its
//! [`key`](Item::key), and each result is in one canonical form (one item per key, newest
//! first, then by key). So the usual laws hold as equalities:
//!
//! - `union` and `intersect` are associative, commutative and idempotent;
//! - `diff(a, a)` is empty, and `diff(a, b)` never holds a key of `b`;
//! - `filter(p, filter(q, a)) = filter(all[p, q], a)`;
//! - `filter(p, union(a, b)) = union(filter(p, a), filter(p, b))`.
//!
//! When two feeds hold the same key, the item of the feed whose slug sorts first is kept, so the
//! choice does not depend on the order of the operands.

use crate::{ALERTS, Item, time};
use alerts_core::{Severity, Uf, min_severity, only_ufs};
use alerts_feed::{Entry, FeedError, FeedMeta, Status};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A rule: the items of a composed feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    /// The items of one feed, by slug.
    Feed { feed: String },
    /// The official alerts, each as one item that carries its CAP alert.
    Alerts,
    /// The result of another rule, by slug. A rule cannot reach itself.
    Rule { rule: String },
    /// The items of any operand. `union []` is empty.
    Union { of: Vec<Expr> },
    /// The items whose key is in every operand. `intersect []` is empty.
    Intersect { of: Vec<Expr> },
    /// The items of `left` whose key is not in `right`.
    Diff { left: Box<Expr>, right: Box<Expr> },
    /// The items for which `when` holds.
    Filter { when: Pred, of: Box<Expr> },
    /// Each item changed by `with`. The key and the time do not change, and an item that
    /// carries an alert is never changed: a channel never rewrites an alert.
    Map { with: MapOp, of: Box<Expr> },
    /// One item per title (case and spaces ignored): the newest one.
    Dedupe { of: Box<Expr> },
    /// The `n` newest items.
    Take { n: usize, of: Box<Expr> },
}

/// A test on one item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "is", rename_all = "snake_case", deny_unknown_fields)]
pub enum Pred {
    /// The text is in the field, case ignored.
    Contains {
        field: TextField,
        text: String,
    },
    /// The item time is at or after `time` (RFC 3339 or RFC 822). An item without a time fails.
    Since {
        time: String,
    },
    /// The item comes from one of these feeds (`alerts` names the official alerts).
    From {
        feeds: Vec<String>,
    },
    /// The item carries an alert at this severity or above (the `min_severity` stage).
    MinSeverity {
        severity: Severity,
    },
    /// The item carries an alert that touches one of these UFs (the `only_ufs` stage).
    Ufs {
        ufs: Vec<Uf>,
    },
    Not {
        p: Box<Pred>,
    },
    /// Every test holds. `all []` always holds.
    All {
        of: Vec<Pred>,
    },
    /// At least one test holds. `any []` never holds.
    Any {
        of: Vec<Pred>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextField {
    Title,
    Summary,
    /// The title or the summary.
    Any,
}

/// A change to one item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapOp {
    /// Puts `text` before the title, for example `[G1] `.
    PrefixTitle { text: String },
    /// Removes the HTML tags of the summary.
    StripHtml,
    /// Keeps the first `chars` characters of the summary, and adds `…` when it cut.
    Truncate { chars: usize },
}

/// What a rule reads: the stored items of each feed, the official alerts, and the other rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    pub feeds: BTreeMap<String, Vec<Item>>,
    pub alerts: Vec<Item>,
    pub rules: BTreeMap<String, Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    UnknownFeed(String),
    UnknownRule(String),
    /// The rules in the order they reach each other; the last one is the first one again.
    Cycle(Vec<String>),
    /// A `since` time that is not RFC 3339 or RFC 822.
    BadTime(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::UnknownFeed(s) => write!(f, "o feed {s} não existe"),
            EvalError::UnknownRule(s) => write!(f, "a regra {s} não existe"),
            EvalError::Cycle(path) => write!(f, "ciclo de regras: {}", path.join(" → ")),
            EvalError::BadTime(t) => write!(f, "tempo inválido: {t}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// What an expression names directly, without following its rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Refs {
    pub feeds: BTreeSet<String>,
    pub rules: BTreeSet<String>,
    /// The expression reads the official alerts.
    pub alerts: bool,
}

impl Expr {
    pub fn refs(&self) -> Refs {
        let mut r = Refs::default();
        self.walk(&mut |e| match e {
            Expr::Feed { feed } => {
                r.feeds.insert(feed.clone());
            }
            Expr::Rule { rule } => {
                r.rules.insert(rule.clone());
            }
            Expr::Alerts => r.alerts = true,
            _ => {}
        });
        r
    }

    fn walk(&self, f: &mut impl FnMut(&Expr)) {
        f(self);
        match self {
            Expr::Feed { .. } | Expr::Alerts | Expr::Rule { .. } => {}
            Expr::Union { of } | Expr::Intersect { of } => of.iter().for_each(|e| e.walk(f)),
            Expr::Diff { left, right } => {
                left.walk(f);
                right.walk(f);
            }
            Expr::Filter { of, .. }
            | Expr::Map { of, .. }
            | Expr::Dedupe { of }
            | Expr::Take { of, .. } => of.walk(f),
        }
    }
}

/// The items of `expr` in canonical form. The function is pure: the same rule and the same
/// environment give the same items.
pub fn eval(expr: &Expr, env: &Env) -> Result<Vec<Item>, EvalError> {
    go(expr, env, &mut Vec::new())
}

fn go(expr: &Expr, env: &Env, stack: &mut Vec<String>) -> Result<Vec<Item>, EvalError> {
    Ok(match expr {
        Expr::Feed { feed } => canonical(
            env.feeds
                .get(feed)
                .ok_or_else(|| EvalError::UnknownFeed(feed.clone()))?
                .clone(),
        ),
        Expr::Alerts => canonical(env.alerts.clone()),
        Expr::Rule { rule } => {
            if let Some(i) = stack.iter().position(|r| r == rule) {
                let mut path = stack[i..].to_vec();
                path.push(rule.clone());
                return Err(EvalError::Cycle(path));
            }
            let e = env
                .rules
                .get(rule)
                .ok_or_else(|| EvalError::UnknownRule(rule.clone()))?;
            stack.push(rule.clone());
            let items = go(e, env, stack)?;
            stack.pop();
            items
        }
        Expr::Union { of } => {
            let mut all = Vec::new();
            for e in of {
                all.extend(go(e, env, stack)?);
            }
            canonical(all)
        }
        Expr::Intersect { of } => {
            let sets = of
                .iter()
                .map(|e| go(e, env, stack))
                .collect::<Result<Vec<_>, _>>()?;
            let keys = |s: &Vec<Item>| s.iter().map(|i| i.key.clone()).collect::<BTreeSet<_>>();
            let common = sets
                .iter()
                .map(keys)
                .reduce(|a, b| a.intersection(&b).cloned().collect())
                .unwrap_or_default();
            canonical(
                sets.into_iter()
                    .flatten()
                    .filter(|i| common.contains(&i.key))
                    .collect(),
            )
        }
        Expr::Diff { left, right } => {
            let drop: BTreeSet<_> = go(right, env, stack)?.into_iter().map(|i| i.key).collect();
            let mut items = go(left, env, stack)?;
            items.retain(|i| !drop.contains(&i.key));
            items
        }
        Expr::Filter { when, of } => {
            check_pred(when)?;
            let mut items = go(of, env, stack)?;
            items.retain(|i| holds(when, i));
            items
        }
        Expr::Map { with, of } => go(of, env, stack)?
            .into_iter()
            .map(|i| apply(with, i))
            .collect(),
        Expr::Dedupe { of } => {
            let mut seen = BTreeSet::new();
            let mut items = go(of, env, stack)?;
            items.retain(|i| seen.insert(fold(&i.title)));
            items
        }
        Expr::Take { n, of } => {
            let mut items = go(of, env, stack)?;
            items.truncate(*n);
            items
        }
    })
}

/// One item per key (the winner of [`wins`]), newest first, then by key. Items without a
/// time come last.
fn canonical(items: Vec<Item>) -> Vec<Item> {
    let mut by_key: BTreeMap<String, Item> = BTreeMap::new();
    for i in items {
        match by_key.get(&i.key) {
            Some(old) if !wins(&i, old) => {}
            _ => {
                by_key.insert(i.key.clone(), i);
            }
        }
    }
    let mut out: Vec<Item> = by_key.into_values().collect();
    out.sort_by(order);
    out
}

fn order(a: &Item, b: &Item) -> Ordering {
    b.updated.cmp(&a.updated).then_with(|| a.key.cmp(&b.key))
}

/// On a key clash, the item of the feed whose slug sorts first wins; the other fields break a
/// tie, so the winner never depends on the order of the operands.
fn wins(a: &Item, b: &Item) -> bool {
    (&a.feed, &a.title, &a.summary, &a.link, &a.updated)
        < (&b.feed, &b.title, &b.summary, &b.link, &b.updated)
}

fn check_pred(p: &Pred) -> Result<(), EvalError> {
    match p {
        Pred::Since { time } if time::normalize(time).is_none() => {
            Err(EvalError::BadTime(time.clone()))
        }
        Pred::Not { p } => check_pred(p),
        Pred::All { of } | Pred::Any { of } => of.iter().try_for_each(check_pred),
        _ => Ok(()),
    }
}

/// Lowercase, with each run of spaces as one space.
fn fold(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn holds(p: &Pred, i: &Item) -> bool {
    match p {
        Pred::Contains { field, text } => {
            let text = fold(text);
            let has = |s: &str| fold(s).contains(&text);
            match field {
                TextField::Title => has(&i.title),
                TextField::Summary => has(&i.summary),
                TextField::Any => has(&i.title) || has(&i.summary),
            }
        }
        Pred::Since { time } => match (time::normalize(time), &i.updated) {
            (Some(t), Some(u)) => *u >= t,
            _ => false,
        },
        Pred::From { feeds } => feeds.contains(&i.feed),
        Pred::MinSeverity { severity } => i
            .alert
            .clone()
            .is_some_and(|a| min_severity(*severity)(a).is_some()),
        Pred::Ufs { ufs } => i
            .alert
            .clone()
            .is_some_and(|a| only_ufs(ufs.clone())(a).is_some()),
        Pred::Not { p } => !holds(p, i),
        Pred::All { of } => of.iter().all(|p| holds(p, i)),
        Pred::Any { of } => of.iter().any(|p| holds(p, i)),
    }
}

fn apply(op: &MapOp, mut i: Item) -> Item {
    if i.alert.is_some() {
        return i;
    }
    match op {
        MapOp::PrefixTitle { text } => i.title = format!("{text}{}", i.title),
        MapOp::StripHtml => i.summary = strip_html(&i.summary),
        MapOp::Truncate { chars } => {
            if i.summary.chars().count() > *chars {
                i.summary = i.summary.chars().take(*chars).collect::<String>() + "…";
            }
        }
    }
    i
}

/// The text outside `<...>`, with the spaces folded. It is for a summary, not a sanitizer:
/// the Atom output escapes the text anyway.
fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The Atom feed of the items, through the Atom + CAP 1.2 channel. An item that carries an
/// alert holds it as CAP 1.2 with status `Actual`. `updated` is the feed time, and the time of
/// an item that has none.
pub fn atom(meta: &FeedMeta, items: &[Item], updated: &str) -> Result<String, FeedError> {
    let entries: Vec<Entry<'_>> = items
        .iter()
        .map(|i| Entry {
            id: entry_id(meta, i),
            title: &i.title,
            link: i.link.as_deref(),
            updated: i.updated.as_deref().unwrap_or(updated),
            summary: &i.summary,
            alert: i.alert.as_ref().map(|a| (a, Status::Actual)),
        })
        .collect();
    alerts_feed::atom_entries(meta, &entries, updated)
}

/// The key when it is already a URI; else the key under the feed id, as `alerts-feed` does for
/// an alert.
fn entry_id(meta: &FeedMeta, i: &Item) -> String {
    let key: String = i
        .key
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if ["http://", "https://", "urn:", "tag:"]
        .iter()
        .any(|p| key.starts_with(p))
    {
        key
    } else if i.feed == ALERTS {
        format!("{}:{key}", meta.id)
    } else {
        format!("{}:{}:{key}", meta.id, i.feed)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use alerts_core::{Alert, Hazard};
    use serde_json::json;

    fn item(feed: &str, key: &str, title: &str, updated: Option<&str>) -> Item {
        Item {
            feed: feed.into(),
            key: key.into(),
            title: title.into(),
            link: None,
            summary: format!("<p>Resumo de {title}</p>"),
            updated: updated.map(Into::into),
            alert: None,
        }
    }

    fn alert(id: &str, uf: Uf, severity: Severity) -> Item {
        Item::from_alert(&Alert {
            id: id.into(),
            source: "defesa-civil-idap".into(),
            hazard: Hazard::Flood,
            severity,
            ufs: vec![uf],
            headline: format!("Alerta {id}"),
            area: "Blumenau (SC)".into(),
            sent: "2026-10-22T12:00:00-03:00".into(),
            url: None,
        })
    }

    fn t(h: u32) -> Option<String> {
        Some(format!("2026-10-22T{h:02}:00:00+00:00"))
    }

    fn env() -> Env {
        let mut e = Env::default();
        let mut g1 = vec![
            item("g1", "a", "Chuva em Blumenau", None),
            item("g1", "b", "Enchente no Vale", None),
            item("g1", "c", "Futebol", None),
        ];
        g1[0].updated = t(10);
        g1[1].updated = t(12);
        let mut uol = vec![
            item("uol", "b", "Enchente no Vale do Itajaí", None),
            item("uol", "d", "CHUVA  em blumenau", None),
        ];
        uol[1].updated = t(9);
        e.feeds.insert("g1".into(), g1);
        e.feeds.insert("uol".into(), uol);
        e.feeds.insert("vazio".into(), vec![]);
        e.alerts = vec![
            alert("1", Uf::Sc, Severity::Severe),
            alert("2", Uf::Sp, Severity::Minor),
        ];
        e
    }

    fn feed(s: &str) -> Expr {
        Expr::Feed { feed: s.into() }
    }

    fn keys(e: &Expr, env: &Env) -> Vec<String> {
        eval(e, env).unwrap().into_iter().map(|i| i.key).collect()
    }

    fn parse(v: serde_json::Value) -> Expr {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn json_form() {
        let e = parse(json!({"op": "take", "n": 5, "of": {"op": "union", "of": [
            {"op": "feed", "feed": "g1"},
            {"op": "filter", "when": {"is": "min_severity", "severity": "Severe"},
             "of": {"op": "alerts"}}
        ]}}));
        let back: Expr = serde_json::from_value(serde_json::to_value(&e).unwrap()).unwrap();
        assert_eq!(back, e);
        assert!(
            serde_json::from_value::<Expr>(json!({"op": "feed", "feed": "g1", "x": 1})).is_err()
        );
        assert!(serde_json::from_value::<Expr>(json!({"op": "zip"})).is_err());
    }

    #[test]
    fn union_is_a_set_union() {
        let env = env();
        let u = Expr::Union {
            of: vec![feed("g1"), feed("uol")],
        };
        // Newest first, then the items without a time by key.
        assert_eq!(keys(&u, &env), ["b", "a", "d", "c"]);
        // The clash on key b keeps the g1 item, whatever the order of the operands.
        let b = eval(&u, &env).unwrap().remove(0);
        assert_eq!(b.feed, "g1");
        let swapped = Expr::Union {
            of: vec![feed("uol"), feed("g1")],
        };
        assert_eq!(eval(&u, &env), eval(&swapped, &env));
        assert_eq!(
            keys(&Expr::Union { of: vec![] }, &env),
            Vec::<String>::new()
        );
    }

    #[test]
    fn set_laws() {
        let env = env();
        let (a, b, c) = (feed("g1"), feed("uol"), Expr::Alerts);
        let union = |of: Vec<Expr>| Expr::Union { of };
        let inter = |of: Vec<Expr>| Expr::Intersect { of };
        let same = |x: &Expr, y: &Expr| assert_eq!(eval(x, &env), eval(y, &env), "{x:?} {y:?}");
        for op in [union, inter] {
            same(
                &op(vec![a.clone(), b.clone()]),
                &op(vec![b.clone(), a.clone()]),
            );
            same(
                &op(vec![op(vec![a.clone(), b.clone()]), c.clone()]),
                &op(vec![a.clone(), op(vec![b.clone(), c.clone()])]),
            );
            same(&op(vec![a.clone(), a.clone()]), &a);
        }
        assert_eq!(keys(&inter(vec![a.clone(), b.clone()]), &env), ["b"]);
        let diff = |l: &Expr, r: &Expr| Expr::Diff {
            left: Box::new(l.clone()),
            right: Box::new(r.clone()),
        };
        assert!(keys(&diff(&a, &a), &env).is_empty());
        assert_eq!(keys(&diff(&a, &b), &env), ["a", "c"]);
    }

    #[test]
    fn filter_laws() {
        let env = env();
        let p = Pred::Contains {
            field: TextField::Title,
            text: "chuva".into(),
        };
        let q = Pred::Since {
            time: "2026-10-22T09:30:00Z".into(),
        };
        let filter = |when: &Pred, of: Expr| Expr::Filter {
            when: when.clone(),
            of: Box::new(of),
        };
        let both = Expr::Union {
            of: vec![feed("g1"), feed("uol")],
        };
        assert_eq!(
            eval(&filter(&p, filter(&q, both.clone())), &env),
            eval(
                &filter(
                    &Pred::All {
                        of: vec![p.clone(), q.clone()]
                    },
                    both.clone()
                ),
                &env
            )
        );
        assert_eq!(
            eval(&filter(&p, both.clone()), &env),
            eval(
                &Expr::Union {
                    of: vec![filter(&p, feed("g1")), filter(&p, feed("uol"))]
                },
                &env
            )
        );
        // "chuva" matches "CHUVA  em blumenau": case and spaces are ignored.
        assert_eq!(keys(&filter(&p, both), &env), ["a", "d"]);
    }

    #[test]
    fn alert_predicates_reuse_the_harness() {
        let env = env();
        let severe = Expr::Filter {
            when: Pred::MinSeverity {
                severity: Severity::Severe,
            },
            of: Box::new(Expr::Union {
                of: vec![Expr::Alerts, feed("g1")],
            }),
        };
        let items = eval(&severe, &env).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].alert.as_ref().map(|a| a.id.as_str()), Some("1"));
        let south = Expr::Filter {
            when: Pred::Ufs {
                ufs: Uf::V0.to_vec(),
            },
            of: Box::new(Expr::Alerts),
        };
        assert_eq!(eval(&south, &env).unwrap().len(), 1);
        let from = Expr::Filter {
            when: Pred::Not {
                p: Box::new(Pred::From {
                    feeds: vec![ALERTS.into()],
                }),
            },
            of: Box::new(Expr::Union {
                of: vec![Expr::Alerts, feed("uol")],
            }),
        };
        assert_eq!(keys(&from, &env), ["d", "b"]);
    }

    #[test]
    fn map_dedupe_take() {
        let env = env();
        let mapped = Expr::Map {
            with: MapOp::PrefixTitle {
                text: "[SC] ".into(),
            },
            of: Box::new(Expr::Union {
                of: vec![feed("g1"), Expr::Alerts],
            }),
        };
        let items = eval(&mapped, &env).unwrap();
        assert!(
            items
                .iter()
                .filter(|i| i.alert.is_none())
                .all(|i| i.title.starts_with("[SC] "))
        );
        // A map never rewrites an alert.
        assert!(
            items
                .iter()
                .filter(|i| i.alert.is_some())
                .all(|i| i.title.starts_with("Alerta"))
        );

        let stripped = Expr::Map {
            with: MapOp::StripHtml,
            of: Box::new(feed("g1")),
        };
        assert_eq!(
            eval(&stripped, &env).unwrap()[0].summary,
            "Resumo de Enchente no Vale"
        );
        let cut = Expr::Map {
            with: MapOp::Truncate { chars: 3 },
            of: Box::new(stripped),
        };
        assert_eq!(eval(&cut, &env).unwrap()[0].summary, "Res…");

        let dedupe = Expr::Dedupe {
            of: Box::new(Expr::Union {
                of: vec![feed("g1"), feed("uol")],
            }),
        };
        // "CHUVA  em blumenau" (09:00) is the same title as "Chuva em Blumenau" (10:00).
        assert_eq!(keys(&dedupe, &env), ["b", "a", "c"]);
        let take = Expr::Take {
            n: 2,
            of: Box::new(dedupe),
        };
        assert_eq!(keys(&take, &env), ["b", "a"]);
    }

    #[test]
    fn rules_compose_and_cycles_fail() {
        let mut env = env();
        env.rules.insert(
            "chuva".into(),
            Expr::Filter {
                when: Pred::Contains {
                    field: TextField::Any,
                    text: "chuva".into(),
                },
                of: Box::new(feed("g1")),
            },
        );
        env.rules.insert(
            "tudo".into(),
            Expr::Union {
                of: vec![
                    Expr::Rule {
                        rule: "chuva".into(),
                    },
                    feed("vazio"),
                ],
            },
        );
        assert_eq!(
            keys(
                &Expr::Rule {
                    rule: "tudo".into()
                },
                &env
            ),
            ["a"]
        );

        env.rules
            .insert("x".into(), Expr::Rule { rule: "y".into() });
        env.rules
            .insert("y".into(), Expr::Rule { rule: "x".into() });
        assert_eq!(
            eval(&Expr::Rule { rule: "x".into() }, &env),
            Err(EvalError::Cycle(vec!["x".into(), "y".into(), "x".into()]))
        );
        assert_eq!(
            eval(&feed("nada"), &env),
            Err(EvalError::UnknownFeed("nada".into()))
        );
        assert_eq!(
            eval(
                &Expr::Rule {
                    rule: "nada".into()
                },
                &env
            ),
            Err(EvalError::UnknownRule("nada".into()))
        );
        let bad = Expr::Filter {
            when: Pred::Since {
                time: "ontem".into(),
            },
            of: Box::new(feed("vazio")),
        };
        assert_eq!(eval(&bad, &env), Err(EvalError::BadTime("ontem".into())));
        let r = env.rules["tudo"].refs();
        assert_eq!(r.feeds.into_iter().collect::<Vec<_>>(), ["vazio"]);
        assert_eq!(r.rules.into_iter().collect::<Vec<_>>(), ["chuva"]);
        assert!(!r.alerts && Expr::Alerts.refs().alerts);
    }

    #[test]
    fn atom_embeds_cap_for_alert_items() {
        let env = env();
        let items = eval(
            &Expr::Union {
                of: vec![Expr::Alerts, feed("g1")],
            },
            &env,
        )
        .unwrap();
        let meta = FeedMeta {
            id: "urn:piabrota-alerts:regra:sul".into(),
            title: "Sul".into(),
        };
        let x = atom(&meta, &items, "2026-10-22T15:00:00+00:00").unwrap();
        assert_eq!(x.matches("<entry>").count(), 5);
        assert_eq!(
            x.matches("<content type=\"application/cap+xml\">").count(),
            2
        );
        assert!(x.contains("<status>Actual</status>"));
        assert!(x.contains("<id>urn:piabrota-alerts:regra:sul:g1:a</id>"));
        assert!(x.contains(
            "<id>urn:piabrota-alerts:regra:sul:piabrota-alerts-defesa-civil-idap-1</id>"
        ));
        // An item without a time takes the feed time.
        assert!(x.contains(
            "<updated>2026-10-22T15:00:00+00:00</updated><summary>&lt;p&gt;Resumo de Futebol"
        ));
    }
}
