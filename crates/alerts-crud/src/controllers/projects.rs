//! Controller of projects: the default CRUD, plus a check that the parent exists.

use super::{Resource, parse, require};
use crate::Db;
use crate::error::Result;
use crate::model;
use crate::models::areas::Area;
use crate::models::projects::{Project, ProjectParams, SPEC};
use serde_json::Value;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        ..Resource::crud::<Project>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: ProjectParams = parse(params)?;
    require::<Area>(db, "area_id", params.area_id, "A área")?;
    Ok(serde_json::to_value(model::create::<Project>(
        db, &params,
    )?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: ProjectParams = parse(params)?;
    require::<Area>(db, "area_id", params.area_id, "A área")?;
    Ok(serde_json::to_value(model::update::<Project>(
        db, id, &params,
    )?)?)
}
