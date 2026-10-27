//! The schema, one migration per file, applied in the order of [`MIGRATIONS`]. A migration
//! never changes after it is merged: a schema change is a new file. The name is the
//! `m<date>_<seq>_<what>` form of Loco.

mod m20261027_000001_create_sources;
mod m20261027_000002_create_channels;
mod m20261027_000003_create_apps;
mod m20261027_000004_create_alerts;
mod m20261027_000005_create_users;
mod m20261027_000006_create_areas;
mod m20261027_000007_create_projects;
mod m20261027_000008_create_tasks;
mod m20261027_000009_create_feeds;
mod m20261027_000010_create_rules;
mod m20261027_000011_create_feed_items;
// scaffold: migration modules

/// `(name, SQL)` pairs, oldest first.
pub const MIGRATIONS: &[(&str, &str)] = &[
    m20261027_000001_create_sources::MIGRATION,
    m20261027_000002_create_channels::MIGRATION,
    m20261027_000003_create_apps::MIGRATION,
    m20261027_000004_create_alerts::MIGRATION,
    m20261027_000005_create_users::MIGRATION,
    m20261027_000006_create_areas::MIGRATION,
    m20261027_000007_create_projects::MIGRATION,
    m20261027_000008_create_tasks::MIGRATION,
    m20261027_000009_create_feeds::MIGRATION,
    m20261027_000010_create_rules::MIGRATION,
    m20261027_000011_create_feed_items::MIGRATION,
    // scaffold: migrations
];
