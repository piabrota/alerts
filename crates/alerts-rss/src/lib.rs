//! RSS combinator: feeds as values, and rules that compose them.
//!
//! A feed is a list of [`Item`]s. [`parse`] reads an RSS 2.0, RSS 1.0 or Atom document into
//! items. A rule is an [`Expr`], a small algebra over item sets: union, intersection, difference,
//! filter, map, dedupe and take, with feeds, the official alerts and other rules as its leaves.
//! [`eval`] is a pure function from a rule and an [`Env`] to items, and [`atom`] writes the
//! result with the Atom + CAP 1.2 channel of `alerts-feed`.
//!
//! The crate does no I/O. A fetcher (`alerts rss-fetch`) reads the feeds over HTTP and stores
//! their items; `alerts-crud` stores the rules and evaluates them on the stored items.

mod expr;
mod parse;
pub mod time;

pub use expr::{Env, EvalError, Expr, MapOp, Pred, Refs, TextField, atom, eval};
pub use parse::{ParseError, parse};

use alerts_core::Alert;
use serde::{Deserialize, Serialize};

/// The feed slug of the items that come from the official alerts. No feed can use it.
pub const ALERTS: &str = "alerts";

/// One entry of a feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// The slug of the feed the item came from.
    pub feed: String,
    /// The identity of the item in every set operation: its link, else its guid or Atom id,
    /// else its title.
    pub key: String,
    pub title: String,
    pub link: Option<String>,
    pub summary: String,
    /// The UTC form of [`time::normalize`], or `None` when the feed gave no valid time.
    pub updated: Option<String>,
    /// The official alert this item carries, unchanged. Only the [`ALERTS`] leaf sets it.
    pub alert: Option<Alert>,
}

impl Item {
    /// An official alert as an item. The Atom output embeds the alert as CAP 1.2.
    pub fn from_alert(a: &Alert) -> Self {
        Item {
            feed: ALERTS.into(),
            key: alerts_feed::identifier(a),
            title: a.headline.clone(),
            link: a.url.clone(),
            summary: a.area.clone(),
            updated: time::normalize(&a.sent),
            alert: Some(a.clone()),
        }
    }
}
