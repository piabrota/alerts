pub const MIGRATION: (&str, &str) = (
    "m20261027_000007_create_projects",
    "CREATE TABLE projects (
        id INTEGER PRIMARY KEY,
        area_id INTEGER NOT NULL REFERENCES areas (id) ON DELETE CASCADE,
        slug TEXT NOT NULL,
        name TEXT NOT NULL,
        done INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        UNIQUE (slug, area_id)
    )",
);
