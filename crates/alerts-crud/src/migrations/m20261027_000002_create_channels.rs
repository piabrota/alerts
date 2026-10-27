pub const MIGRATION: (&str, &str) = (
    "m20261027_000002_create_channels",
    "CREATE TABLE channels (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        kind TEXT NOT NULL,
        target_env TEXT NOT NULL,
        enabled INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    )",
);
