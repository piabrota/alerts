//! The SQLite database of the admin interface. Opening it runs every pending migration.

use crate::error::Result;
use crate::migrations::MIGRATIONS;
use rusqlite::{Connection, params};
use std::path::Path;

pub type Db = Connection;

pub fn open(path: &Path) -> Result<Db> {
    let db = Connection::open(path)?;
    migrate(&db)?;
    Ok(db)
}

pub fn open_in_memory() -> Result<Db> {
    let db = Connection::open_in_memory()?;
    migrate(&db)?;
    Ok(db)
}

/// Applies, in order, each migration that `_migrations` does not list. Each one runs in its own
/// transaction, so a failed migration leaves the database at the one before it.
pub fn migrate(db: &Db) -> Result<()> {
    db.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS _migrations (
             name TEXT PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         );",
    )?;
    for (name, sql) in MIGRATIONS {
        let done: bool = db.query_row(
            "SELECT EXISTS (SELECT 1 FROM _migrations WHERE name = ?1)",
            params![name],
            |r| r.get(0),
        )?;
        if !done {
            db.execute_batch(&format!(
                "BEGIN; {sql}; INSERT INTO _migrations (name) VALUES ('{name}'); COMMIT;"
            ))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn migrations_run_once() {
        let db = open_in_memory().unwrap();
        migrate(&db).unwrap();
        let n: i64 = db
            .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, MIGRATIONS.len() as i64);
    }
}
