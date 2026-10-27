//! The alerts desktop app. Tauri 2 holds the window and the SQLite database; every command
//! calls the `alerts-crud` controllers, so the interface and an HTTP server would share one
//! code path.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use alerts_crud::controllers::{Request, dispatch, specs};
use alerts_crud::schema::ModuleSpec;
use alerts_crud::{Db, Error};
use serde_json::Value;
use std::sync::Mutex;
use tauri::Manager;

struct Store(Mutex<Db>);

/// The specs of every resource. The interface draws one module per spec.
#[tauri::command]
fn modules() -> Vec<&'static ModuleSpec> {
    specs()
}

/// One CRUD request, for example `{"op": "list", "resource": "apps"}`.
#[tauri::command]
fn crud(store: tauri::State<'_, Store>, request: Request) -> Result<Value, Error> {
    let db = store.0.lock().map_err(|_| Error::Db {
        message: "o banco ficou inconsistente depois de um erro; reinicie o app".into(),
    })?;
    dispatch(&db, request)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .setup(|app| {
            // ALERTS_DB names another database file, for example a copy to test with.
            let path = match std::env::var_os("ALERTS_DB") {
                Some(p) => p.into(),
                None => {
                    let dir = app.path().app_data_dir()?;
                    std::fs::create_dir_all(&dir)?;
                    dir.join("alerts.db")
                }
            };
            app.manage(Store(Mutex::new(alerts_crud::open(&path)?)));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![modules, crud])
        .run(tauri::generate_context!())?;
    Ok(())
}
