//! The schema, one migration per file, applied in the order of [`MIGRATIONS`]. A migration
//! never changes after it is merged: a schema change is a new file. The name is the
//! `m<date>_<seq>_<what>` form of Loco.

mod m20261027_000001_create_sources;
mod m20261027_000002_create_channels;
mod m20261027_000003_create_apps;
mod m20261027_000004_create_alerts;
// scaffold: migration modules

/// `(name, SQL)` pairs, oldest first.
pub const MIGRATIONS: &[(&str, &str)] = &[
    m20261027_000001_create_sources::MIGRATION,
    m20261027_000002_create_channels::MIGRATION,
    m20261027_000003_create_apps::MIGRATION,
    m20261027_000004_create_alerts::MIGRATION,
    // scaffold: migrations
];
