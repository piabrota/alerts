//! Controller of feed items: the default CRUD. The spec allows only read and delete.

use super::Resource;
use crate::models::feed_items::{FeedItem, SPEC};

pub fn resource() -> Resource {
    Resource::crud::<FeedItem>(&SPEC)
}
