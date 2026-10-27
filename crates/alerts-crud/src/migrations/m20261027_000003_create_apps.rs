pub const MIGRATION: (&str, &str) = (
    "m20261027_000003_create_apps",
    "CREATE TABLE apps (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        hazards TEXT NOT NULL,
        ufs TEXT NOT NULL,
        sources TEXT NOT NULL,
        min_severity TEXT NOT NULL,
        channels TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    )",
);
