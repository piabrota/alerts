#![allow(clippy::unwrap_used)]

use alerts_core::{Alert, Hazard, Severity, Uf};
use alerts_crud::controllers::{Request, dispatch};
use alerts_crud::models::alerts::record;
use alerts_crud::models::feed_items::record_items;
use alerts_crud::{Db, Error, open_in_memory};
use alerts_rss::Item;
use serde_json::{Value, json};

fn call(db: &Db, v: Value) -> Result<Value, Error> {
    dispatch(db, serde_json::from_value::<Request>(v).unwrap())
}

fn create(db: &Db, resource: &str, params: Value) -> i64 {
    call(
        db,
        json!({"op": "create", "resource": resource, "params": params}),
    )
    .unwrap()["id"]
        .as_i64()
        .unwrap()
}

fn fields(r: Result<Value, Error>) -> Vec<String> {
    let Err(Error::Invalid { errors }) = r else {
        panic!("expected a validation error, got {r:?}");
    };
    errors.into_iter().map(|e| e.field).collect()
}

/// A user with one area, one project and one task. Returns the four ids.
fn tree(db: &Db) -> (i64, i64, i64, i64) {
    let user = create(db, "users", json!({"slug": "hoffmann", "name": "Hoffmann"}));
    let area = create(
        db,
        "areas",
        json!({"user_id": user, "slug": "enchentes-sc", "name": "Enchentes em SC"}),
    );
    let project = create(
        db,
        "projects",
        json!({"area_id": area, "slug": "cheia-2026", "name": "Cheia de 2026", "done": false}),
    );
    let task = create(
        db,
        "tasks",
        json!({"project_id": project, "title": "Ler os boletins", "done": true}),
    );
    (user, area, project, task)
}

fn feed(slug: &str, scope: &str, scope_id: i64) -> Value {
    json!({
        "slug": slug, "title": slug, "url": format!("https://example.org/{slug}.xml"),
        "scope": scope, "scope_id": scope_id, "enabled": true
    })
}

fn rule(slug: &str, expr: Value, scope: &str, scope_id: i64) -> Value {
    json!({
        "slug": slug, "title": format!("Regra {slug}"), "expr": expr.to_string(),
        "scope": scope, "scope_id": scope_id
    })
}

fn item(feed: &str, key: &str, title: &str, updated: &str) -> Item {
    Item {
        feed: feed.into(),
        key: key.into(),
        title: title.into(),
        link: Some(format!("https://example.org/{key}")),
        summary: "Resumo.".into(),
        updated: Some(updated.into()),
        alert: None,
    }
}

#[test]
fn para_tree_checks_its_parents() {
    let db = open_in_memory().unwrap();
    let (user, area, project, task) = tree(&db);
    let r = call(
        &db,
        json!({"op": "create", "resource": "areas", "params": {
            "user_id": 99, "slug": "x", "name": "X"
        }}),
    );
    assert_eq!(fields(r), ["user_id"]);
    // The same area slug is fine for another user, not for the same one.
    let other = create(&db, "users", json!({"slug": "ana", "name": "Ana"}));
    create(
        &db,
        "areas",
        json!({"user_id": other, "slug": "enchentes-sc", "name": "Outra"}),
    );
    let r = call(
        &db,
        json!({"op": "create", "resource": "areas", "params": {
            "user_id": user, "slug": "enchentes-sc", "name": "De novo"
        }}),
    );
    assert_eq!(fields(r), ["slug"]);

    create(&db, "feeds", feed("g1-sc", "area", area));
    create(&db, "feeds", feed("defesa-sc", "task", task));
    create(
        &db,
        "rules",
        rule(
            "sul",
            json!({"op": "feed", "feed": "g1-sc"}),
            "project",
            project,
        ),
    );
    let outline = call(
        &db,
        json!({"op": "action", "resource": "users", "id": user, "action": "para"}),
    )
    .unwrap();
    assert_eq!(
        outline.as_str().unwrap(),
        format!(
            "Hoffmann (hoffmann)\n\
             Área enchentes-sc [{area}]: Enchentes em SC\n\
             \x20 feeds: g1-sc\n\
             \x20 Projeto cheia-2026 [{project}]: Cheia de 2026\n\
             \x20   regras: sul\n\
             \x20   Tarefa [{task}]: Ler os boletins (concluída)\n\
             \x20     feeds: defesa-sc\n"
        )
    );
}

#[test]
fn feeds_check_owner_and_slug() {
    let db = open_in_memory().unwrap();
    let (_, area, _, _) = tree(&db);
    let r = call(
        &db,
        json!({"op": "create", "resource": "feeds", "params": feed("g1", "project", 99)}),
    );
    assert_eq!(fields(r), ["scope_id"]);
    let mut bad = feed("alerts", "area", area);
    bad["url"] = json!("http://example.org/feed");
    bad["scope"] = json!("user");
    let r = call(
        &db,
        json!({"op": "create", "resource": "feeds", "params": bad}),
    );
    assert_eq!(fields(r), ["slug", "url", "scope"]);
}

#[test]
fn rules_are_checked_before_they_are_stored() {
    let db = open_in_memory().unwrap();
    let (_, area, _, _) = tree(&db);
    create(&db, "feeds", feed("g1-sc", "area", area));
    let post = |p: Value| {
        call(
            &db,
            json!({"op": "create", "resource": "rules", "params": p}),
        )
    };

    let mut not_json = rule("a", json!(null), "area", area);
    not_json["expr"] = json!("{\"op\": \"zip\"}");
    assert_eq!(fields(post(not_json)), ["expr"]);
    assert_eq!(
        fields(post(rule(
            "a",
            json!({"op": "feed", "feed": "nada"}),
            "area",
            area
        ))),
        ["expr"]
    );
    assert_eq!(
        fields(post(rule(
            "a",
            json!({"op": "rule", "rule": "a"}),
            "area",
            area
        ))),
        ["expr"]
    );

    let a = create(
        &db,
        "rules",
        rule("a", json!({"op": "feed", "feed": "g1-sc"}), "area", area),
    );
    create(
        &db,
        "rules",
        rule("b", json!({"op": "rule", "rule": "a"}), "area", area),
    );
    // a -> b -> a is a cycle.
    let r = call(
        &db,
        json!({"op": "update", "resource": "rules", "id": a,
               "params": rule("a", json!({"op": "rule", "rule": "b"}), "area", area)}),
    );
    assert_eq!(fields(r), ["expr"]);
    // Rule b reads rule a, and rule a reads feed g1-sc: neither can go.
    let r = call(&db, json!({"op": "delete", "resource": "rules", "id": a}));
    assert_eq!(fields(r), ["slug"]);
    let r = call(&db, json!({"op": "delete", "resource": "feeds", "id": 1}));
    assert_eq!(fields(r), ["slug"]);
}

#[test]
fn rule_atom_composes_feeds_and_alerts() {
    let db = open_in_memory().unwrap();
    let (_, area, project, _) = tree(&db);
    create(&db, "feeds", feed("g1-sc", "area", area));
    create(&db, "feeds", feed("nsc", "project", project));
    record_items(
        &db,
        "g1-sc",
        &[
            item(
                "g1-sc",
                "a",
                "Chuva forte em Blumenau",
                "2026-10-22T10:00:00+00:00",
            ),
            item("g1-sc", "b", "Futebol", "2026-10-22T11:00:00+00:00"),
        ],
    )
    .unwrap();
    // The fetcher reads the same item twice: one record.
    record_items(
        &db,
        "nsc",
        &[item(
            "nsc",
            "c",
            "Rio Itajaí-Açu sobe",
            "2026-10-22T09:00:00+00:00",
        )],
    )
    .unwrap();
    record_items(
        &db,
        "nsc",
        &[item(
            "nsc",
            "c",
            "Rio Itajaí-Açu sobe 2 m",
            "2026-10-22T09:30:00+00:00",
        )],
    )
    .unwrap();
    let alert = |id: &str, uf, severity| Alert {
        id: id.into(),
        source: "defesa-civil-idap".into(),
        hazard: Hazard::Flood,
        severity,
        ufs: vec![uf],
        headline: format!("Alerta {id}."),
        area: "Blumenau (SC)".into(),
        sent: "2026-10-22T12:00:00-03:00".into(),
        url: None,
    };
    record(&db, &alert("1", Uf::Sc, Severity::Severe)).unwrap();
    record(&db, &alert("2", Uf::Sp, Severity::Severe)).unwrap();

    create(
        &db,
        "rules",
        rule(
            "chuva",
            json!({"op": "filter",
                   "when": {"is": "contains", "field": "title", "text": "chuva"},
                   "of": {"op": "feed", "feed": "g1-sc"}}),
            "area",
            area,
        ),
    );
    let sul = create(
        &db,
        "rules",
        rule(
            "sul",
            json!({"op": "take", "n": 10, "of": {"op": "union", "of": [
                {"op": "rule", "rule": "chuva"},
                {"op": "map", "with": {"do": "prefix_title", "text": "[NSC] "},
                 "of": {"op": "feed", "feed": "nsc"}},
                {"op": "filter", "when": {"is": "ufs", "ufs": ["PR", "SC", "RS"]},
                 "of": {"op": "alerts"}}
            ]}}),
            "project",
            project,
        ),
    );
    let x = call(
        &db,
        json!({"op": "action", "resource": "rules", "id": sul, "action": "atom"}),
    )
    .unwrap();
    let x = x.as_str().unwrap();
    assert_eq!(x.matches("<entry>").count(), 3, "{x}");
    assert!(x.contains("<title>Chuva forte em Blumenau</title>"));
    assert!(x.contains("<title>[NSC] Rio Itajaí-Açu sobe 2 m</title>"));
    assert!(!x.contains("Futebol"));
    // The SC alert is in the feed as CAP 1.2; the SP alert is not.
    assert_eq!(
        x.matches("<content type=\"application/cap+xml\">").count(),
        1
    );
    assert!(x.contains("<identifier>piabrota-alerts-defesa-civil-idap-1</identifier>"));
    // The feed time is the newest item time: the alert, sent 12:00 at -03:00.
    assert!(x.contains("<updated>2026-10-22T15:00:00+00:00</updated><author>"));

    let items = call(&db, json!({"op": "list", "resource": "feed_items"})).unwrap();
    assert_eq!(items.as_array().map(Vec::len), Some(3));
    let r = call(
        &db,
        json!({"op": "create", "resource": "feed_items", "params": {}}),
    );
    assert!(matches!(r, Err(Error::NotAllowed { .. })), "{r:?}");
}

#[test]
fn deleting_a_user_deletes_the_tree() {
    let db = open_in_memory().unwrap();
    let (user, area, project, task) = tree(&db);
    create(&db, "feeds", feed("g1-sc", "area", area));
    create(&db, "feeds", feed("nsc", "task", task));
    create(
        &db,
        "rules",
        rule("sul", json!({"op": "alerts"}), "project", project),
    );
    record_items(
        &db,
        "nsc",
        &[item("nsc", "c", "Rio sobe", "2026-10-22T09:00:00+00:00")],
    )
    .unwrap();

    // Deleting the task deletes its feed and that feed's items, not the area feed.
    call(
        &db,
        json!({"op": "delete", "resource": "tasks", "id": task}),
    )
    .unwrap();
    let count = |table: &str| -> i64 {
        db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!((count("feeds"), count("feed_items")), (1, 0));

    call(
        &db,
        json!({"op": "delete", "resource": "users", "id": user}),
    )
    .unwrap();
    for table in ["areas", "projects", "tasks", "feeds", "rules", "feed_items"] {
        assert_eq!(count(table), 0, "{table}");
    }
}
