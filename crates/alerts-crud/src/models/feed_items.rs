//! The items that the fetcher read from the feeds. The interface reads and deletes them; only
//! the fetcher writes them, with [`record_items`].

use crate::error::Result;
use crate::model;
use crate::schema::{FieldKind, ModuleSpec, Op, field};
use crate::validate::{FieldError, Validate};
use rusqlite::params;

model! {
    /// One item of a feed, as the feed stated it. `(feed, key)` is unique.
    FeedItem, FeedItemParams in "feed_items" {
        feed: String,
        key: String,
        title: String,
        link: Option<String>,
        summary: String,
        updated: Option<String>,
    }
}

/// The interface cannot create or edit an item: an item comes only from its feed.
impl Validate for FeedItemParams {
    fn validate(&self) -> std::result::Result<(), Vec<FieldError>> {
        Err(vec![FieldError {
            field: "key".into(),
            message: "Um item vem só do seu feed.".into(),
        }])
    }
}

impl From<FeedItem> for alerts_rss::Item {
    fn from(r: FeedItem) -> Self {
        alerts_rss::Item {
            feed: r.feed,
            key: r.key,
            title: r.title,
            link: r.link,
            summary: r.summary,
            updated: r.updated,
            alert: None,
        }
    }
}

/// Stores the items that the fetcher read from `feed`, in one transaction. The same
/// `(feed, key)` updates the stored item. Returns how many items it wrote.
pub fn record_items(db: &crate::Db, feed: &str, items: &[alerts_rss::Item]) -> Result<usize> {
    let tx = db.unchecked_transaction()?;
    for i in items {
        tx.execute(
            "INSERT INTO feed_items (feed, key, title, link, summary, updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (feed, key) DO UPDATE SET
                 title = excluded.title, link = excluded.link, summary = excluded.summary,
                 updated = excluded.updated,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
            params![feed, i.key, i.title, i.link, i.summary, i.updated],
        )?;
    }
    tx.commit()?;
    Ok(items.len())
}

/// The stored items of one feed.
pub fn of_feed(db: &crate::Db, feed: &str) -> Result<Vec<alerts_rss::Item>> {
    let mut stmt = db.prepare("SELECT * FROM feed_items WHERE feed = ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![feed], |r| {
        <FeedItem as crate::model::Model>::from_row(r)
    })?;
    Ok(rows
        .map(|r| r.map(Into::into))
        .collect::<rusqlite::Result<_>>()?)
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "feed_items",
    label: "Itens de feed",
    singular: "Item de feed",
    about: "Os itens que o leitor de feeds guardou. A interface não cria nem edita um item.",
    fields: &[
        field("title", "Título", FieldKind::LongText, "", true),
        field("feed", "Feed", FieldKind::Text, "", true),
        field("updated", "Atualizado em", FieldKind::Text, "", true),
        field("link", "Endereço", FieldKind::Url, "", false),
        field("summary", "Resumo", FieldKind::LongText, "", false),
        field("key", "Chave", FieldKind::Text, "", false),
    ],
    ops: Op::READ_DELETE,
    actions: &[],
};
