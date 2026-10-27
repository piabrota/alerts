//! Controller of tasks: the default CRUD, plus a check that the parent exists.

use super::{Resource, parse, require};
use crate::Db;
use crate::error::Result;
use crate::model;
use crate::models::projects::Project;
use crate::models::tasks::{SPEC, Task, TaskParams};
use serde_json::Value;

pub fn resource() -> Resource {
    Resource {
        create,
        update,
        ..Resource::crud::<Task>(&SPEC)
    }
}

fn create(db: &Db, params: Value) -> Result<Value> {
    let params: TaskParams = parse(params)?;
    require::<Project>(db, "project_id", params.project_id, "O projeto")?;
    Ok(serde_json::to_value(model::create::<Task>(db, &params)?)?)
}

fn update(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: TaskParams = parse(params)?;
    require::<Project>(db, "project_id", params.project_id, "O projeto")?;
    Ok(serde_json::to_value(model::update::<Task>(
        db, id, &params,
    )?)?)
}
