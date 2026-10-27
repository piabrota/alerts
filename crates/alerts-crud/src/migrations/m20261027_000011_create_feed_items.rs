pub const MIGRATION: (&str, &str) = (
    "m20261027_000011_create_feed_items",
    "CREATE TABLE feed_items (
        id INTEGER PRIMARY KEY,
        feed TEXT NOT NULL REFERENCES feeds (slug) ON DELETE CASCADE ON UPDATE CASCADE,
        key TEXT NOT NULL,
        title TEXT NOT NULL,
        link TEXT,
        summary TEXT NOT NULL,
        updated TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        UNIQUE (feed, key)
    )",
);
