//! Controller of users: the default CRUD, plus the `para` action, the outline of the areas,
//! projects and tasks of a user with the feeds and rules of each.

use super::Resource;
use crate::Db;
use crate::error::{Error, Result};
use crate::model::{self, Model};
use crate::models::areas::Area;
use crate::models::feeds::Feed;
use crate::models::projects::Project;
use crate::models::rules::Rule;
use crate::models::tasks::Task;
use crate::models::users::{SPEC, User};
use std::fmt::Write;

pub fn resource() -> Resource {
    Resource {
        action,
        ..Resource::crud::<User>(&SPEC)
    }
}

fn action(db: &Db, id: i64, action: &str) -> Result<String> {
    match action {
        "para" => para(db, id),
        _ => Err(Error::UnknownAction {
            resource: User::TABLE.into(),
            action: action.into(),
        }),
    }
}

/// The PARA tree of one user, as indented text.
pub fn para(db: &Db, id: i64) -> Result<String> {
    let user = model::find::<User>(db, id)?;
    let areas = model::list::<Area>(db)?;
    let projects = model::list::<Project>(db)?;
    let tasks = model::list::<Task>(db)?;
    let feeds = model::list::<Feed>(db)?;
    let rules = model::list::<Rule>(db)?;
    let owned = |scope: &str, id: i64, indent: &str, out: &mut String| {
        let f: Vec<_> = feeds
            .iter()
            .filter(|f| f.scope == scope && f.scope_id == id)
            .map(|f| f.slug.as_str())
            .collect();
        let r: Vec<_> = rules
            .iter()
            .filter(|r| r.scope == scope && r.scope_id == id)
            .map(|r| r.slug.as_str())
            .collect();
        if !f.is_empty() {
            let _ = writeln!(out, "{indent}feeds: {}", f.join(", "));
        }
        if !r.is_empty() {
            let _ = writeln!(out, "{indent}regras: {}", r.join(", "));
        }
    };
    // Writing to a String cannot fail, so the writeln! results are ignored.
    let mut out = String::new();
    let _ = writeln!(out, "{} ({})", user.name, user.slug);
    for a in areas.iter().filter(|a| a.user_id == user.id) {
        let _ = writeln!(out, "Área {} [{}]: {}", a.slug, a.id, a.name);
        owned("area", a.id, "  ", &mut out);
        for p in projects.iter().filter(|p| p.area_id == a.id) {
            let done = if p.done { " (concluído)" } else { "" };
            let _ = writeln!(out, "  Projeto {} [{}]: {}{done}", p.slug, p.id, p.name);
            owned("project", p.id, "    ", &mut out);
            for t in tasks.iter().filter(|t| t.project_id == p.id) {
                let done = if t.done { " (concluída)" } else { "" };
                let _ = writeln!(out, "    Tarefa [{}]: {}{done}", t.id, t.title);
                owned("task", t.id, "      ", &mut out);
            }
        }
    }
    Ok(out)
}
