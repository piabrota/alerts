pub const MIGRATION: (&str, &str) = (
    "m20261027_000001_create_sources",
    "CREATE TABLE sources (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        agency TEXT NOT NULL,
        url TEXT NOT NULL,
        poll_every TEXT NOT NULL,
        enabled INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    )",
);
