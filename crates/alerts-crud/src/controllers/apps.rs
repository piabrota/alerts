//! Controller of apps: the default CRUD, plus two checks that need the database (each source
//! and each channel exists) and the `app_toml` action.

use super::{Resource, parse};
use crate::Db;
use crate::error::{Error, Result};
use crate::model::{self, Model};
use crate::models::apps::{App, AppParams, SPEC};
use crate::models::channels::Channel;
use crate::models::sources::Source;
use crate::validate::FieldError;
use serde_json::Value;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        action,
        ..Resource::crud::<App>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: AppParams = parse(params)?;
    check_refs(db, &params)?;
    Ok(serde_json::to_value(model::create::<App>(db, &params)?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: AppParams = parse(params)?;
    check_refs(db, &params)?;
    Ok(serde_json::to_value(model::update::<App>(
        db, id, &params,
    )?)?)
}

fn action(db: &Db, id: i64, action: &str) -> Result<String> {
    match action {
        "app_toml" => {
            let app = model::find::<App>(db, id)?;
            let sources = model::list::<Source>(db)?;
            app.to_toml(&sources).map_err(|e| Error::Json {
                message: e.to_string(),
            })
        }
        _ => Err(Error::UnknownAction {
            resource: App::TABLE.into(),
            action: action.into(),
        }),
    }
}

fn check_refs(db: &Db, p: &AppParams) -> Result<()> {
    let mut errors = Vec::new();
    for slug in &p.sources {
        if model::find_by::<Source>(db, "slug", slug)?.is_none() {
            errors.push(missing("sources", "A fonte", slug));
        }
    }
    for slug in &p.channels {
        if model::find_by::<Channel>(db, "slug", slug)?.is_none() {
            errors.push(missing("channels", "O canal", slug));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::invalid(errors))
    }
}

fn missing(field: &str, what: &str, slug: &str) -> FieldError {
    FieldError {
        field: field.into(),
        message: format!("{what} {slug} não existe."),
    }
}
