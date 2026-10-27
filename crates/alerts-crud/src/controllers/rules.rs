//! Controller of rules: the default CRUD, plus the checks that need the database (the owner
//! exists, every feed and rule the expression names exists, no rule reaches itself) and the
//! `atom` action, which evaluates the rule on the stored items.

use super::{Resource, check_scope, parse};
use crate::Db;
use crate::error::{Error, Result};
use crate::model::{self, Model};
use crate::models::alerts::AlertRecord;
use crate::models::feed_items::of_feed;
use crate::models::feeds::Feed;
use crate::models::rules::{Rule, RuleParams, SPEC};
use crate::validate::FieldError;
use alerts_feed::FeedMeta;
use alerts_rss::{Env, Expr, Item};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        delete,
        action,
        ..Resource::crud::<Rule>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: RuleParams = parse(params)?;
    check(db, None, &params)?;
    Ok(serde_json::to_value(model::create::<Rule>(db, &params)?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: RuleParams = parse(params)?;
    let old = model::find::<Rule>(db, id)?;
    check(db, Some(&old), &params)?;
    if old.slug != params.slug {
        unused(db, &old.slug)?;
    }
    Ok(serde_json::to_value(model::update::<Rule>(
        db, id, &params,
    )?)?)
}

fn delete(db: &Db, id: i64) -> Result<()> {
    let old = model::find::<Rule>(db, id)?;
    unused(db, &old.slug)?;
    model::delete::<Rule>(db, id)
}

fn action(db: &Db, id: i64, action: &str) -> Result<String> {
    match action {
        "atom" => atom(db, id),
        _ => Err(Error::UnknownAction {
            resource: Rule::TABLE.into(),
            action: action.into(),
        }),
    }
}

/// The items of the rule `id`, from the stored items of its feeds and the stored alerts.
pub fn items(db: &Db, id: i64) -> Result<Vec<Item>> {
    let rule = model::find::<Rule>(db, id)?;
    let rules = all_rules(db)?;
    // Load only what the rule reaches, through the rules it names.
    let mut env = Env::default();
    let mut todo = vec![rule.slug.clone()];
    let mut seen = std::collections::BTreeSet::new();
    let mut alerts = false;
    while let Some(slug) = todo.pop() {
        let Some(e) = rules.get(&slug) else { continue };
        if !seen.insert(slug) {
            continue;
        }
        let r = e.refs();
        alerts |= r.alerts;
        for f in r.feeds {
            if let std::collections::btree_map::Entry::Vacant(v) = env.feeds.entry(f) {
                let items = of_feed(db, v.key())?;
                v.insert(items);
            }
        }
        todo.extend(r.rules);
    }
    // A feed with no stored item is still a feed.
    for f in model::list::<Feed>(db)? {
        env.feeds.entry(f.slug).or_default();
    }
    if alerts {
        env.alerts = model::list::<AlertRecord>(db)?
            .into_iter()
            .map(|a| Item::from_alert(&a.into()))
            .collect();
    }
    env.rules = rules;
    alerts_rss::eval(&Expr::Rule { rule: rule.slug }, &env).map_err(|e| Error::Json {
        message: e.to_string(),
    })
}

/// The rule as an Atom feed, through the Atom + CAP 1.2 channel. The feed time is the time of
/// its newest item.
pub fn atom(db: &Db, id: i64) -> Result<String> {
    let rule = model::find::<Rule>(db, id)?;
    let items = items(db, id)?;
    let meta = FeedMeta {
        id: format!("urn:piabrota-alerts:rule:{}", rule.slug),
        title: rule.title,
    };
    let updated = items
        .iter()
        .filter_map(|i| i.updated.clone())
        .max()
        .unwrap_or_else(|| "1970-01-01T00:00:00+00:00".into());
    alerts_rss::atom(&meta, &items, &updated).map_err(|e| Error::Json {
        message: e.to_string(),
    })
}

fn all_rules(db: &Db) -> Result<BTreeMap<String, Expr>> {
    model::list::<Rule>(db)?
        .into_iter()
        .map(|r| {
            let e = r.parse_expr()?;
            Ok((r.slug, e))
        })
        .collect()
}

/// Evaluates the new expression on empty feeds: this finds an unknown feed or rule, a bad
/// time, and a rule that reaches itself, before anything is stored.
fn check(db: &Db, old: Option<&Rule>, p: &RuleParams) -> Result<()> {
    check_scope(db, &p.scope, p.scope_id)?;
    let Ok(expr) = p.parse_expr() else {
        // The model validation names the error.
        return Ok(());
    };
    let mut env = Env::default();
    for f in model::list::<Feed>(db)? {
        env.feeds.insert(f.slug, Vec::new());
    }
    env.rules = all_rules(db)?;
    if let Some(old) = old {
        env.rules.remove(&old.slug);
    }
    env.rules.insert(p.slug.clone(), expr);
    alerts_rss::eval(
        &Expr::Rule {
            rule: p.slug.clone(),
        },
        &env,
    )
    .map(|_| ())
    .map_err(|e| {
        Error::invalid(vec![FieldError {
            field: "expr".into(),
            message: format!("A regra não pode ser avaliada: {e}."),
        }])
    })
}

/// Fails when another rule reads the rule `slug`.
fn unused(db: &Db, slug: &str) -> Result<()> {
    let users: Vec<_> = model::list::<Rule>(db)?
        .into_iter()
        .filter(|r| r.slug != slug && r.parse_expr().is_ok_and(|e| e.refs().rules.contains(slug)))
        .map(|r| r.slug)
        .collect();
    if users.is_empty() {
        Ok(())
    } else {
        Err(Error::invalid(vec![FieldError {
            field: "slug".into(),
            message: format!("As regras {} leem a regra {slug}.", users.join(", ")),
        }]))
    }
}
