//! Controller of sources: the default CRUD.

use super::Resource;
use crate::models::sources::{SPEC, Source};

pub fn resource() -> Resource {
    Resource::crud::<Source>(&SPEC)
}
