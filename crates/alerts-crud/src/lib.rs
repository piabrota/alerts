//! Loco-style CRUD for the alerts admin interface.
//!
//! The layout copies [Loco](https://loco.rs): `models/` holds the data and its validation,
//! `controllers/` holds the operations, `migrations/` holds the schema, one file each. A
//! scaffold (`just generate scaffold <name> <field:kind>...`) writes all three for a new
//! resource. The transport is not here: the desktop app calls [`controllers::dispatch`] from a
//! Tauri command, and an HTTP server could call the same function.
//!
//! The resources configure apps (sources, channels, apps). Alerts are read-only: only a source
//! module writes them, through [`models::alerts::record`]. The interface never creates an alert.

pub mod controllers;
pub mod db;
pub mod error;
pub mod migrations;
pub mod model;
pub mod models;
pub mod schema;
pub mod validate;

pub use db::{Db, open, open_in_memory};
pub use error::Error;
