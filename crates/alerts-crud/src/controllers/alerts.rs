//! Controller of alerts: list, show and delete. `SPEC.ops` refuses create and update.

use super::Resource;
use crate::models::alerts::{AlertRecord, SPEC};

pub fn resource() -> Resource {
    Resource::crud::<AlertRecord>(&SPEC)
}
