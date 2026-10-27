// A feed belongs to one area, project or task (`scope`, `scope_id`). SQLite cannot hold a
// foreign key to one of three tables, so a trigger per table deletes the feeds of a deleted
// owner. A cascade from a user or an area fires the triggers too.
pub const MIGRATION: (&str, &str) = (
    "m20261027_000009_create_feeds",
    "CREATE TABLE feeds (
        id INTEGER PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        url TEXT NOT NULL,
        scope TEXT NOT NULL,
        scope_id INTEGER NOT NULL,
        enabled INTEGER NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    CREATE TRIGGER feeds_of_area AFTER DELETE ON areas BEGIN
        DELETE FROM feeds WHERE scope = 'area' AND scope_id = OLD.id;
    END;
    CREATE TRIGGER feeds_of_project AFTER DELETE ON projects BEGIN
        DELETE FROM feeds WHERE scope = 'project' AND scope_id = OLD.id;
    END;
    CREATE TRIGGER feeds_of_task AFTER DELETE ON tasks BEGIN
        DELETE FROM feeds WHERE scope = 'task' AND scope_id = OLD.id;
    END",
);
