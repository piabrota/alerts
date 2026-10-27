#![allow(clippy::unwrap_used)]

use alerts_core::{Alert, Hazard, Severity, Uf};
use alerts_crud::controllers::{Request, dispatch};
use alerts_crud::models::alerts::record;
use alerts_crud::models::apps::App;
use alerts_crud::{Error, open_in_memory};
use serde_json::{Value, json};

fn req(v: Value) -> Request {
    serde_json::from_value(v).unwrap()
}

fn seed(db: &alerts_crud::Db) {
    dispatch(
        db,
        req(json!({"op": "create", "resource": "sources", "params": {
            "slug": "defesa-civil-idap", "name": "IDAP", "agency": "Defesa Civil",
            "url": "https://idap.example/feed", "poll_every": "every 5m", "enabled": true
        }})),
    )
    .unwrap();
    dispatch(
        db,
        req(json!({"op": "create", "resource": "channels", "params": {
            "slug": "discord", "kind": "discord", "target_env": "DISCORD_WEBHOOK_URL", "enabled": true
        }})),
    )
    .unwrap();
}

fn app_params() -> Value {
    json!({
        "slug": "sul-enchentes", "name": "Enchentes no Sul", "hazards": ["Flood"],
        "ufs": ["PR", "SC", "RS"], "sources": ["defesa-civil-idap"],
        "min_severity": "Moderate", "channels": ["discord"]
    })
}

#[test]
fn crud_round_trip() {
    let db = open_in_memory().unwrap();
    seed(&db);
    let created = dispatch(
        &db,
        req(json!({"op": "create", "resource": "apps", "params": app_params()})),
    )
    .unwrap();
    let id = created["id"].as_i64().unwrap();
    assert_eq!(created["ufs"], json!(["PR", "SC", "RS"]));

    let mut p = app_params();
    p["name"] = json!("Enchentes PR, SC e RS");
    let updated = dispatch(
        &db,
        req(json!({"op": "update", "resource": "apps", "id": id, "params": p})),
    )
    .unwrap();
    assert_eq!(updated["name"], "Enchentes PR, SC e RS");

    let list = dispatch(&db, req(json!({"op": "list", "resource": "apps"}))).unwrap();
    assert_eq!(list.as_array().map(Vec::len), Some(1));

    dispatch(
        &db,
        req(json!({"op": "delete", "resource": "apps", "id": id})),
    )
    .unwrap();
    let gone = dispatch(
        &db,
        req(json!({"op": "show", "resource": "apps", "id": id})),
    );
    assert!(matches!(gone, Err(Error::NotFound { .. })));
}

#[test]
fn validation_names_the_fields() {
    let db = open_in_memory().unwrap();
    let r = dispatch(
        &db,
        req(json!({"op": "create", "resource": "channels", "params": {
            "slug": "Discord", "kind": "discord",
            "target_env": "https://discord.com/api/webhooks/1", "enabled": true
        }})),
    );
    let Err(Error::Invalid { errors }) = r else {
        panic!("expected a validation error, got {r:?}");
    };
    let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
    assert_eq!(fields, ["slug", "target_env"]);
}

#[test]
fn duplicate_slug_is_a_field_error() {
    let db = open_in_memory().unwrap();
    seed(&db);
    let r = dispatch(
        &db,
        req(json!({"op": "create", "resource": "channels", "params": {
            "slug": "discord", "kind": "feed", "target_env": "FEED_PATH", "enabled": true
        }})),
    );
    let Err(Error::Invalid { errors }) = r else {
        panic!("expected a validation error, got {r:?}");
    };
    assert_eq!(errors[0].field, "slug");
}

#[test]
fn app_refs_must_exist() {
    let db = open_in_memory().unwrap();
    let r = dispatch(
        &db,
        req(json!({"op": "create", "resource": "apps", "params": app_params()})),
    );
    let Err(Error::Invalid { errors }) = r else {
        panic!("expected a validation error, got {r:?}");
    };
    let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
    assert_eq!(fields, ["sources", "channels"]);
}

#[test]
fn app_exports_toml_and_a_harness() {
    let db = open_in_memory().unwrap();
    seed(&db);
    let created = dispatch(
        &db,
        req(json!({"op": "create", "resource": "apps", "params": app_params()})),
    )
    .unwrap();
    let id = created["id"].as_i64().unwrap();
    let toml = dispatch(
        &db,
        req(json!({"op": "action", "resource": "apps", "id": id, "action": "app_toml"})),
    )
    .unwrap();
    let toml = toml.as_str().unwrap();
    assert!(toml.contains(r#"name = "sul-enchentes""#), "{toml}");
    assert!(toml.contains(r#"hazards = ["flood"]"#), "{toml}");
    assert!(
        toml.contains(r#"stages = ["only_ufs", "min_severity:moderate"]"#),
        "{toml}"
    );
    assert!(
        toml.contains("[schedule]\ndefesa-civil-idap = \"every 5m\""),
        "{toml}"
    );

    let app: App = serde_json::from_value(created).unwrap();
    let h = app.harness();
    assert!(h.run(alert("a", vec![Uf::Rs], Severity::Severe)).is_some());
    assert!(h.run(alert("b", vec![Uf::Sp], Severity::Severe)).is_none());
    assert!(h.run(alert("c", vec![Uf::Pr], Severity::Minor)).is_none());
}

fn alert(id: &str, ufs: Vec<Uf>, severity: Severity) -> Alert {
    Alert {
        id: id.into(),
        source: "defesa-civil-idap".into(),
        hazard: Hazard::Flood,
        severity,
        ufs,
        headline: "Teste.".into(),
        area: "Porto Alegre".into(),
        sent: "2026-10-22T12:00:00-03:00".into(),
        url: None,
    }
}

#[test]
fn alerts_are_read_only() {
    let db = open_in_memory().unwrap();
    let r = dispatch(
        &db,
        req(json!({"op": "create", "resource": "alerts", "params": {}})),
    );
    assert!(matches!(r, Err(Error::NotAllowed { .. })), "{r:?}");

    let a = alert("idap-1", vec![Uf::Rs], Severity::Severe);
    let first = record(&db, &a).unwrap();
    let mut again = a.clone();
    again.severity = Severity::Extreme;
    let second = record(&db, &again).unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.severity, Severity::Extreme);
    assert_eq!(Alert::from(second), again);

    let list = dispatch(&db, req(json!({"op": "list", "resource": "alerts"}))).unwrap();
    assert_eq!(list[0]["severity"], "Extreme");
    let r = dispatch(
        &db,
        req(json!({"op": "update", "resource": "alerts", "id": first.id, "params": {}})),
    );
    assert!(matches!(r, Err(Error::NotAllowed { .. })), "{r:?}");
}

#[test]
fn unknown_resource_and_bad_params() {
    let db = open_in_memory().unwrap();
    let r = dispatch(&db, req(json!({"op": "list", "resource": "users"})));
    assert!(matches!(r, Err(Error::UnknownResource { .. })), "{r:?}");
    let r = dispatch(
        &db,
        req(json!({"op": "create", "resource": "sources", "params": {"slug": "x"}})),
    );
    assert!(matches!(r, Err(Error::Invalid { .. })), "{r:?}");
}

#[test]
fn every_spec_field_is_a_model_column() {
    for r in alerts_crud::controllers::resources() {
        for f in r.spec.fields {
            assert!(
                r.columns.contains(&f.name),
                "{}.{} is not a column",
                r.spec.resource,
                f.name
            );
        }
    }
}
