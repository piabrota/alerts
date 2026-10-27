// A rule belongs to one area, project or task, as a feed does (see the feeds migration).
pub const MIGRATION: (&str, &str) = (
    "m20261027_000010_create_rules",
    "CREATE TABLE rules (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        expr TEXT NOT NULL,
        scope TEXT NOT NULL,
        scope_id INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    CREATE TRIGGER rules_of_area AFTER DELETE ON areas BEGIN
        DELETE FROM rules WHERE scope = 'area' AND scope_id = OLD.id;
    END;
    CREATE TRIGGER rules_of_project AFTER DELETE ON projects BEGIN
        DELETE FROM rules WHERE scope = 'project' AND scope_id = OLD.id;
    END;
    CREATE TRIGGER rules_of_task AFTER DELETE ON tasks BEGIN
        DELETE FROM rules WHERE scope = 'task' AND scope_id = OLD.id;
    END",
);
