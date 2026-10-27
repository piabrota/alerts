//! The alerts that the sources delivered. The interface reads and deletes them; only a source
//! module writes them, with [`record`]. Nobody creates an alert by hand.

use crate::error::Result;
use crate::model;
use crate::schema::{FieldKind, HAZARDS, ModuleSpec, Op, SEVERITIES, UFS, field};
use crate::validate::{FieldError, Validate};
use alerts_core::{Hazard, Severity, Uf};
use rusqlite::params;

model! {
    /// One alert, as the source stated it. `alert_id` is the id that the source gave.
    AlertRecord, AlertParams in "alerts" {
        alert_id: String,
        source: String,
        hazard: Hazard,
        severity: Severity,
        ufs: Vec<Uf>,
        headline: String,
        area: String,
        sent: String,
        url: Option<String>,
    }
}

/// The params of an alert are never valid input: the interface cannot create or edit one.
impl Validate for AlertParams {
    fn validate(&self) -> std::result::Result<(), Vec<FieldError>> {
        Err(vec![FieldError {
            field: "alert_id".into(),
            message: "Um alerta vem só de uma fonte oficial.".into(),
        }])
    }
}

impl From<AlertRecord> for alerts_core::Alert {
    fn from(r: AlertRecord) -> Self {
        alerts_core::Alert {
            id: r.alert_id,
            source: r.source,
            hazard: r.hazard,
            severity: r.severity,
            ufs: r.ufs,
            headline: r.headline,
            area: r.area,
            sent: r.sent,
            url: r.url,
        }
    }
}

/// Stores an alert from a source. The same `(source, id)` updates the stored alert, so a source
/// that reads the same alert twice keeps one record.
pub fn record(db: &crate::Db, a: &alerts_core::Alert) -> Result<AlertRecord> {
    use crate::model::Column;
    db.execute(
        "INSERT INTO alerts (alert_id, source, hazard, severity, ufs, headline, area, sent, url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT (source, alert_id) DO UPDATE SET
             hazard = excluded.hazard, severity = excluded.severity, ufs = excluded.ufs,
             headline = excluded.headline, area = excluded.area, sent = excluded.sent,
             url = excluded.url, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        params![
            a.id,
            a.source,
            a.hazard.to_value(),
            a.severity.to_value(),
            a.ufs.to_value(),
            a.headline,
            a.area,
            a.sent,
            a.url,
        ],
    )?;
    let id: i64 = db.query_row(
        "SELECT id FROM alerts WHERE source = ?1 AND alert_id = ?2",
        params![a.source, a.id],
        |r| r.get(0),
    )?;
    crate::model::find(db, id)
}

pub const SPEC: ModuleSpec = ModuleSpec {
    resource: "alerts",
    label: "Alertas",
    singular: "Alerta",
    about: "Os alertas que as fontes oficiais entregaram. A interface não cria nem edita um alerta.",
    fields: &[
        field("headline", "Título", FieldKind::LongText, "", true),
        field(
            "severity",
            "Severidade",
            FieldKind::Choice {
                options: SEVERITIES,
            },
            "",
            true,
        ),
        field(
            "hazard",
            "Ameaça",
            FieldKind::Choice { options: HAZARDS },
            "",
            false,
        ),
        field("ufs", "UFs", FieldKind::Many { options: UFS }, "", true),
        field("area", "Área", FieldKind::LongText, "", false),
        field("source", "Fonte", FieldKind::Text, "", true),
        field("sent", "Enviado em", FieldKind::Text, "", true),
        field("alert_id", "Id na fonte", FieldKind::Text, "", false),
        field("url", "Endereço", FieldKind::Url, "", false),
    ],
    ops: Op::READ_DELETE,
    actions: &[],
};
