//! Controller of channels: the default CRUD.

use super::Resource;
use crate::models::channels::{Channel, SPEC};

pub fn resource() -> Resource {
    Resource::crud::<Channel>(&SPEC)
}
