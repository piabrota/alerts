//! The model layer: a [`Model`] is one table, and [`model!`](crate::model!) writes the record
//! struct, its params struct and the table mapping, as Loco's entities do. The free functions
//! ([`list`], [`find`], [`create`], [`update`], [`delete`]) are the default CRUD of every model.

use crate::error::{Error, Result};
use crate::validate::{FieldError, Validate};
use rusqlite::types::{FromSqlError, FromSqlResult, Value, ValueRef};
use rusqlite::{OptionalExtension, params, params_from_iter};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use rusqlite::Row;

/// One table. `id`, `created_at` and `updated_at` are in every table, so `COLUMNS` leaves them
/// out.
pub trait Model: Serialize + Sized {
    const TABLE: &'static str;
    const COLUMNS: &'static [&'static str];
    /// What a create or an update sends: every column except the three above.
    type Params: Validate + Serialize + DeserializeOwned;

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self>;
    /// The params in the order of `COLUMNS`.
    fn values(params: &Self::Params) -> Vec<Value>;
}

/// A Rust type that one column stores.
pub trait Column: Sized {
    fn to_value(&self) -> Value;
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self>;
}

impl Column for String {
    fn to_value(&self) -> Value {
        Value::Text(self.clone())
    }
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(v.as_str()?.to_owned())
    }
}

impl Column for Option<String> {
    fn to_value(&self) -> Value {
        self.clone().map_or(Value::Null, Value::Text)
    }
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
        match v {
            ValueRef::Null => Ok(None),
            v => Ok(Some(v.as_str()?.to_owned())),
        }
    }
}

impl Column for bool {
    fn to_value(&self) -> Value {
        Value::Integer(i64::from(*self))
    }
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(v.as_i64()? != 0)
    }
}

impl Column for i64 {
    fn to_value(&self) -> Value {
        Value::Integer(*self)
    }
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
        v.as_i64()
    }
}

/// A list is one JSON array in a text column.
impl<T: Serialize + DeserializeOwned> Column for Vec<T> {
    fn to_value(&self) -> Value {
        Value::Text(serde_json::to_string(self).unwrap_or_else(|_| "[]".into()))
    }
    fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
        serde_json::from_str(v.as_str()?).map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

/// A unit enum is its serde name in a text column, for example `Moderate`.
macro_rules! enum_column {
    ($($t:ty),*) => {$(
        impl Column for $t {
            fn to_value(&self) -> Value {
                match serde_json::to_value(self) {
                    Ok(serde_json::Value::String(s)) => Value::Text(s),
                    _ => Value::Null,
                }
            }
            fn from_value(v: ValueRef<'_>) -> FromSqlResult<Self> {
                serde_json::from_value(serde_json::Value::String(v.as_str()?.to_owned()))
                    .map_err(|e| FromSqlError::Other(Box::new(e)))
            }
        }
    )*};
}

enum_column!(alerts_core::Severity, alerts_core::Hazard);

/// Reads the column `name` of `row`.
pub fn get<T: Column>(row: &Row<'_>, name: &str) -> rusqlite::Result<T> {
    let i = row.as_ref().column_index(name)?;
    T::from_value(row.get_ref(i)?).map_err(|e| match e {
        FromSqlError::InvalidType => rusqlite::Error::InvalidColumnType(
            i,
            name.to_owned(),
            row.get_ref(i)
                .map_or(rusqlite::types::Type::Null, |v| v.data_type()),
        ),
        e => rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e)),
    })
}

/// Writes a record struct, its params struct and their [`Model`] impl. Each field is one
/// column; its type must be a [`Column`].
///
/// ```ignore
/// model! {
///     /// An agency the app reads alerts from.
///     Source, SourceParams in "sources" {
///         slug: String,
///         enabled: bool,
///     }
/// }
/// ```
#[macro_export]
macro_rules! model {
    (
        $(#[$meta:meta])*
        $name:ident, $params:ident in $table:literal {
            $($(#[$fmeta:meta])* $field:ident: $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            pub id: i64,
            $($(#[$fmeta])* pub $field: $ty,)*
            pub created_at: String,
            pub updated_at: String,
        }

        /// The fields a create or an update sends.
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct $params {
            $($(#[$fmeta])* pub $field: $ty,)*
        }

        impl $crate::model::Model for $name {
            const TABLE: &'static str = $table;
            const COLUMNS: &'static [&'static str] = &[$(stringify!($field)),*];
            type Params = $params;

            fn from_row(row: &$crate::model::Row<'_>) -> rusqlite::Result<Self> {
                Ok(Self {
                    id: $crate::model::get(row, "id")?,
                    $($field: $crate::model::get(row, stringify!($field))?,)*
                    created_at: $crate::model::get(row, "created_at")?,
                    updated_at: $crate::model::get(row, "updated_at")?,
                })
            }

            fn values(p: &Self::Params) -> Vec<rusqlite::types::Value> {
                vec![$($crate::model::Column::to_value(&p.$field)),*]
            }
        }
    };
}

pub fn list<M: Model>(db: &crate::Db) -> Result<Vec<M>> {
    let mut stmt = db.prepare(&format!("SELECT * FROM {} ORDER BY id", M::TABLE))?;
    let rows = stmt.query_map([], |r| M::from_row(r))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn find<M: Model>(db: &crate::Db, id: i64) -> Result<M> {
    db.query_row(
        &format!("SELECT * FROM {} WHERE id = ?1", M::TABLE),
        params![id],
        |r| M::from_row(r),
    )
    .optional()?
    .ok_or_else(|| Error::NotFound {
        resource: M::TABLE.into(),
        id,
    })
}

/// Finds the record whose `column` equals `value`, if any.
pub fn find_by<M: Model>(db: &crate::Db, column: &str, value: &str) -> Result<Option<M>> {
    if !M::COLUMNS.contains(&column) {
        return Ok(None);
    }
    Ok(db
        .query_row(
            &format!("SELECT * FROM {} WHERE {column} = ?1", M::TABLE),
            params![value],
            |r| M::from_row(r),
        )
        .optional()?)
}

pub fn create<M: Model>(db: &crate::Db, params: &M::Params) -> Result<M> {
    params.validate().map_err(Error::invalid)?;
    let placeholders: Vec<_> = (1..=M::COLUMNS.len()).map(|i| format!("?{i}")).collect();
    db.execute(
        &format!(
            "INSERT INTO {} ({}) VALUES ({})",
            M::TABLE,
            M::COLUMNS.join(", "),
            placeholders.join(", ")
        ),
        params_from_iter(M::values(params)),
    )
    .map_err(unique_to_field::<M>)?;
    find(db, db.last_insert_rowid())
}

pub fn update<M: Model>(db: &crate::Db, id: i64, params: &M::Params) -> Result<M> {
    params.validate().map_err(Error::invalid)?;
    let sets: Vec<_> = M::COLUMNS
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{c} = ?{}", i + 1))
        .collect();
    let mut values = M::values(params);
    values.push(Value::Integer(id));
    let n = db
        .execute(
            &format!(
                "UPDATE {} SET {}, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?{}",
                M::TABLE,
                sets.join(", "),
                values.len()
            ),
            params_from_iter(values),
        )
        .map_err(unique_to_field::<M>)?;
    if n == 0 {
        return Err(Error::NotFound {
            resource: M::TABLE.into(),
            id,
        });
    }
    find(db, id)
}

pub fn delete<M: Model>(db: &crate::Db, id: i64) -> Result<()> {
    let n = db.execute(
        &format!("DELETE FROM {} WHERE id = ?1", M::TABLE),
        params![id],
    )?;
    if n == 0 {
        return Err(Error::NotFound {
            resource: M::TABLE.into(),
            id,
        });
    }
    Ok(())
}

/// SQLite reports a duplicate as `UNIQUE constraint failed: <table>.<column>`. The interface
/// shows it next to that column.
fn unique_to_field<M: Model>(e: rusqlite::Error) -> Error {
    let prefix = format!("UNIQUE constraint failed: {}.", M::TABLE);
    match e.to_string().strip_prefix(&prefix) {
        Some(column) => Error::invalid(vec![FieldError {
            field: column.split(',').next().unwrap_or(column).trim().to_owned(),
            message: "Já existe um registro com este valor.".into(),
        }]),
        None => e.into(),
    }
}
