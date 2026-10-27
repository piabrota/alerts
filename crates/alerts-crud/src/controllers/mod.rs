//! The controllers: one [`Resource`] per model, and [`dispatch`], the one entry point of every
//! transport. A controller module starts from [`Resource::crud`] and replaces only the
//! operations that need more logic, as a Loco controller does.

use crate::Db;
use crate::error::{Error, Result};
use crate::model::{self, Model};
use crate::schema::{ModuleSpec, Op};
use serde::Deserialize;
use serde_json::Value;

pub mod alerts;
pub mod apps;
pub mod channels;
pub mod sources;
// scaffold: controllers

/// The operations of one resource, on JSON values.
#[derive(Debug, Clone, Copy)]
pub struct Resource {
    pub spec: &'static ModuleSpec,
    /// The columns of the model, for checks of the spec.
    pub columns: &'static [&'static str],
    pub list: fn(&Db) -> Result<Value>,
    pub show: fn(&Db, i64) -> Result<Value>,
    pub create: fn(&Db, Value) -> Result<Value>,
    pub update: fn(&Db, i64, Value) -> Result<Value>,
    pub delete: fn(&Db, i64) -> Result<()>,
    pub action: fn(&Db, i64, &str) -> Result<String>,
}

impl Resource {
    /// The default CRUD of a model. `spec.ops` still decides which operations run.
    pub fn crud<M: Model>(spec: &'static ModuleSpec) -> Self {
        Resource {
            spec,
            columns: M::COLUMNS,
            list: list::<M>,
            show: show::<M>,
            create: create::<M>,
            update: update::<M>,
            delete: model::delete::<M>,
            action: no_action::<M>,
        }
    }
}

pub fn list<M: Model>(db: &Db) -> Result<Value> {
    Ok(serde_json::to_value(model::list::<M>(db)?)?)
}

pub fn show<M: Model>(db: &Db, id: i64) -> Result<Value> {
    Ok(serde_json::to_value(model::find::<M>(db, id)?)?)
}

pub fn create<M: Model>(db: &Db, params: Value) -> Result<Value> {
    let params: M::Params = parse(params)?;
    Ok(serde_json::to_value(model::create::<M>(db, &params)?)?)
}

pub fn update<M: Model>(db: &Db, id: i64, params: Value) -> Result<Value> {
    let params: M::Params = parse(params)?;
    Ok(serde_json::to_value(model::update::<M>(db, id, &params)?)?)
}

fn no_action<M: Model>(_: &Db, _: i64, action: &str) -> Result<String> {
    Err(Error::UnknownAction {
        resource: M::TABLE.into(),
        action: action.into(),
    })
}

/// Params that do not parse are a field error on the first field serde names, so the form can
/// show it.
pub fn parse<P: serde::de::DeserializeOwned>(params: Value) -> Result<P> {
    serde_json::from_value(params).map_err(|e| {
        let message = e.to_string();
        let field = message
            .split('`')
            .nth(1)
            .filter(|f| !f.contains(' '))
            .unwrap_or("")
            .to_owned();
        Error::invalid(vec![crate::validate::FieldError {
            field,
            message: format!("Valor inválido: {message}"),
        }])
    })
}

/// Every resource, in the order of the menu.
pub fn resources() -> Vec<Resource> {
    vec![
        alerts::resource(),
        apps::resource(),
        sources::resource(),
        channels::resource(),
        // scaffold: resources
    ]
}

/// The specs of every resource, for the interface.
pub fn specs() -> Vec<&'static ModuleSpec> {
    resources().into_iter().map(|r| r.spec).collect()
}

/// One request from a transport. It deserializes from `{"op": "list", "resource": "apps"}`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    List {
        resource: String,
    },
    Show {
        resource: String,
        id: i64,
    },
    Create {
        resource: String,
        params: Value,
    },
    Update {
        resource: String,
        id: i64,
        params: Value,
    },
    Delete {
        resource: String,
        id: i64,
    },
    Action {
        resource: String,
        id: i64,
        action: String,
    },
}

impl Request {
    fn resource(&self) -> &str {
        match self {
            Request::List { resource }
            | Request::Show { resource, .. }
            | Request::Create { resource, .. }
            | Request::Update { resource, .. }
            | Request::Delete { resource, .. }
            | Request::Action { resource, .. } => resource,
        }
    }

    /// The operation this request needs in `spec.ops`. An action needs `show`.
    fn op(&self) -> Op {
        match self {
            Request::List { .. } => Op::List,
            Request::Show { .. } | Request::Action { .. } => Op::Show,
            Request::Create { .. } => Op::Create,
            Request::Update { .. } => Op::Update,
            Request::Delete { .. } => Op::Delete,
        }
    }
}

pub fn dispatch(db: &Db, req: Request) -> Result<Value> {
    let name = req.resource();
    let r = resources()
        .into_iter()
        .find(|r| r.spec.resource == name)
        .ok_or_else(|| Error::UnknownResource {
            resource: name.into(),
        })?;
    let op = req.op();
    if !r.spec.ops.contains(&op) {
        return Err(Error::NotAllowed {
            resource: name.into(),
            op: op.as_str().into(),
        });
    }
    match req {
        Request::List { .. } => (r.list)(db),
        Request::Show { id, .. } => (r.show)(db, id),
        Request::Create { params, .. } => (r.create)(db, params),
        Request::Update { id, params, .. } => (r.update)(db, id, params),
        Request::Delete { id, .. } => (r.delete)(db, id).map(|()| Value::Null),
        Request::Action { id, action, .. } => (r.action)(db, id, &action).map(Value::String),
    }
}
