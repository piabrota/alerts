//! Controller of areas: the default CRUD, plus a check that the parent exists.

use super::{Resource, parse, require};
use crate::Db;
use crate::error::Result;
use crate::model;
use crate::models::areas::{Area, AreaParams, SPEC};
use crate::models::users::User;
use serde_json::Value;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        ..Resource::crud::<Area>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: AreaParams = parse(params)?;
    require::<User>(db, "user_id", params.user_id, "O usuário")?;
    Ok(serde_json::to_value(model::create::<Area>(db, &params)?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: AreaParams = parse(params)?;
    require::<User>(db, "user_id", params.user_id, "O usuário")?;
    Ok(serde_json::to_value(model::update::<Area>(
        db, id, &params,
    )?)?)
}
