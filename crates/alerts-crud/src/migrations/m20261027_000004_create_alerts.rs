pub const MIGRATION: (&str, &str) = (
    "m20261027_000004_create_alerts",
    "CREATE TABLE alerts (
        id INTEGER PRIMARY KEY,
        alert_id TEXT NOT NULL,
        source TEXT NOT NULL,
        hazard TEXT NOT NULL,
        severity TEXT NOT NULL,
        ufs TEXT NOT NULL,
        headline TEXT NOT NULL,
        area TEXT NOT NULL,
        sent TEXT NOT NULL,
        url TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        UNIQUE (source, alert_id)
    )",
);
