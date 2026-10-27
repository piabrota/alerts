pub const MIGRATION: (&str, &str) = (
    "m20261027_000008_create_tasks",
    "CREATE TABLE tasks (
        id INTEGER PRIMARY KEY,
        project_id INTEGER NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
        title TEXT NOT NULL,
        done INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    )",
);
