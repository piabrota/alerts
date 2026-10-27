//! Controller of feeds: the default CRUD, plus two checks that need the database: the owner
//! exists, and no rule loses a feed it reads (by a delete or a new slug).

use super::{Resource, check_scope, parse};
use crate::Db;
use crate::error::{Error, Result};
use crate::model;
use crate::models::feeds::{Feed, FeedParams, SPEC};
use crate::models::rules::Rule;
use crate::validate::FieldError;
use serde_json::Value;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        delete,
        ..Resource::crud::<Feed>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: FeedParams = parse(params)?;
    check_scope(db, &params.scope, params.scope_id)?;
    Ok(serde_json::to_value(model::create::<Feed>(db, &params)?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: FeedParams = parse(params)?;
    check_scope(db, &params.scope, params.scope_id)?;
    let old = model::find::<Feed>(db, id)?;
    if old.slug != params.slug {
        unused(db, &old.slug, "slug")?;
    }
    Ok(serde_json::to_value(model::update::<Feed>(
        db, id, &params,
    )?)?)
}

fn delete(db: &Db, id: i64) -> Result<()> {
    let old = model::find::<Feed>(db, id)?;
    unused(db, &old.slug, "slug")?;
    model::delete::<Feed>(db, id)
}

/// Fails when a rule reads the feed `slug`.
fn unused(db: &Db, slug: &str, field: &str) -> Result<()> {
    let users: Vec<_> = model::list::<Rule>(db)?
        .into_iter()
        .filter(|r| r.parse_expr().is_ok_and(|e| e.refs().feeds.contains(slug)))
        .map(|r| r.slug)
        .collect();
    if users.is_empty() {
        Ok(())
    } else {
        Err(Error::invalid(vec![FieldError {
            field: field.into(),
            message: format!("As regras {} leem o feed {slug}.", users.join(", ")),
        }]))
    }
}
