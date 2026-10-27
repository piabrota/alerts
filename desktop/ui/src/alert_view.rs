//! The alerts as the map page reads them, and the UF borders it draws. Pure functions with no
//! MapLibre and no DOM, so the tests run on the host.
//!
//! The map colors a UF only because an alert names it: `Alert` has no geometry, and the `map`
//! skill forbids a guessed shape. The UF borders come from the IBGE malhas API.

use crate::api::Record;
use alerts_core::{Severity, Uf};
use serde_json::Value;
use std::collections::HashMap;

/// The severities from the highest to the lowest: the order of the legend and of the list.
pub const SEVERITIES: [Severity; 4] = [
    Severity::Extreme,
    Severity::Severe,
    Severity::Moderate,
    Severity::Minor,
];

/// The states of v0: the map opens on them.
pub const V0: &[Uf] = Uf::V0;

/// The feature-state value of a severity on the map: 0 is "no alert".
pub fn rank(s: Severity) -> u8 {
    match s {
        Severity::Minor => 1,
        Severity::Moderate => 2,
        Severity::Severe => 3,
        Severity::Extreme => 4,
    }
}

/// One stored alert, with its fields typed.
#[derive(Debug, Clone, PartialEq)]
pub struct AlertView {
    /// The record id in the database.
    pub key: i64,
    pub alert_id: String,
    pub severity: Severity,
    pub ufs: Vec<Uf>,
    pub headline: String,
    pub area: String,
    pub source: String,
    pub sent: String,
    pub url: Option<String>,
}

impl AlertView {
    /// Reads a record of the `alerts` resource. An error names the field that the map cannot
    /// read; the page counts these records and says so, never hides them in silence.
    pub fn from_record(r: &Record) -> Result<Self, String> {
        let text = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
        let severity = r
            .get("severity")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or("severity")?;
        let ufs = r
            .get("ufs")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or("ufs")?;
        Ok(AlertView {
            key: r.get("id").and_then(Value::as_i64).ok_or("id")?,
            alert_id: text("alert_id"),
            severity,
            ufs,
            headline: text("headline"),
            area: text("area"),
            source: text("source"),
            sent: text("sent"),
            url: r
                .get("url")
                .and_then(Value::as_str)
                .filter(|u| !u.is_empty())
                .map(str::to_owned),
        })
    }

    /// A test alert: every channel starts its headline with `TESTE`.
    pub fn is_test(&self) -> bool {
        self.headline.trim_start().starts_with("TESTE")
    }
}

/// What the page shows. The default shows every alert.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    /// The severities that the legend turned off.
    pub hidden: Vec<Severity>,
    /// Only the alerts that name this UF (a click on the map).
    pub uf: Option<Uf>,
    pub source: Option<String>,
    pub tests: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter {
            hidden: Vec::new(),
            uf: None,
            source: None,
            tests: true,
        }
    }
}

impl Filter {
    pub fn keeps(&self, a: &AlertView) -> bool {
        !self.hidden.contains(&a.severity)
            && self.uf.is_none_or(|uf| a.ufs.contains(&uf))
            && self.source.as_ref().is_none_or(|s| *s == a.source)
            && (self.tests || !a.is_test())
    }

    /// The same filter without the UF: the map colors every UF, and a click picks one.
    pub fn without_uf(&self) -> Filter {
        Filter {
            uf: None,
            ..self.clone()
        }
    }

    pub fn toggle(&mut self, s: Severity) {
        if let Some(i) = self.hidden.iter().position(|h| *h == s) {
            self.hidden.remove(i);
        } else {
            self.hidden.push(s);
        }
    }
}

/// The alerts that `filter` keeps, from the highest severity to the lowest, and the newest
/// first inside one severity.
pub fn visible(alerts: &[AlertView], filter: &Filter) -> Vec<AlertView> {
    let mut v: Vec<AlertView> = alerts.iter().filter(|a| filter.keeps(a)).cloned().collect();
    v.sort_by(|a, b| b.severity.cmp(&a.severity).then(b.sent.cmp(&a.sent)));
    v
}

/// For each UF that an alert names: the highest severity and the number of alerts.
pub fn by_uf(alerts: &[AlertView]) -> HashMap<Uf, (Severity, usize)> {
    let mut m: HashMap<Uf, (Severity, usize)> = HashMap::new();
    for a in alerts {
        for &uf in &a.ufs {
            let e = m.entry(uf).or_insert((a.severity, 0));
            e.0 = e.0.max(a.severity);
            e.1 += 1;
        }
    }
    m
}

pub fn count(alerts: &[AlertView], s: Severity) -> usize {
    alerts.iter().filter(|a| a.severity == s).count()
}

/// The sources of the alerts, sorted, each once.
pub fn sources(alerts: &[AlertView]) -> Vec<String> {
    let mut v: Vec<String> = alerts.iter().map(|a| a.source.clone()).collect();
    v.sort();
    v.dedup();
    v
}

/// `[west, south, east, north]`, in degrees.
pub type BBox = [f64; 4];

/// The bounds of a GeoJSON geometry: every `[lon, lat]` pair, at any depth.
pub fn bbox(geometry: &Value) -> Option<BBox> {
    fn walk(v: &Value, b: &mut Option<BBox>) {
        let Some(items) = v.as_array() else { return };
        if let [Value::Number(x), Value::Number(y), ..] = items.as_slice()
            && let (Some(x), Some(y)) = (x.as_f64(), y.as_f64())
        {
            *b = Some(match *b {
                None => [x, y, x, y],
                Some([w, s, e, n]) => [w.min(x), s.min(y), e.max(x), n.max(y)],
            });
            return;
        }
        for i in items {
            walk(i, b);
        }
    }
    let mut b = None;
    walk(geometry.get("coordinates")?, &mut b);
    b
}

/// The bounds of every UF in `ufs` that `boxes` knows.
pub fn union<'a>(boxes: &HashMap<Uf, BBox>, ufs: impl IntoIterator<Item = &'a Uf>) -> Option<BBox> {
    ufs.into_iter()
        .filter_map(|u| boxes.get(u))
        .fold(None, |acc, &[w, s, e, n]| {
            Some(match acc {
                None => [w, s, e, n],
                Some([w0, s0, e0, n0]) => [w0.min(w), s0.min(s), e0.max(e), n0.max(n)],
            })
        })
}

/// The IBGE UF borders, ready for MapLibre: each feature gets the property `uf` (`"RS"`), and
/// the bounds of each UF. A feature without a known IBGE code is an error: the file changed.
pub fn ufs_geojson(mut raw: Value) -> Result<(Value, HashMap<Uf, BBox>), String> {
    let mut boxes = HashMap::new();
    let features = raw
        .get_mut("features")
        .and_then(Value::as_array_mut)
        .ok_or("the IBGE file has no features")?;
    for f in features.iter_mut() {
        let code = f
            .pointer("/properties/codarea")
            .and_then(Value::as_str)
            .unwrap_or("");
        let uf = Uf::from_ibge(code).ok_or_else(|| format!("unknown IBGE code {code:?}"))?;
        let b = f
            .get("geometry")
            .and_then(bbox)
            .ok_or_else(|| format!("{uf} has no geometry"))?;
        boxes.insert(uf, b);
        if let Some(p) = f.get_mut("properties").and_then(Value::as_object_mut) {
            p.insert("uf".into(), uf.as_str().into());
        }
    }
    if boxes.is_empty() {
        return Err("the IBGE file has no UF".into());
    }
    Ok((raw, boxes))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    fn alert(key: i64, severity: Severity, ufs: &[Uf], headline: &str) -> AlertView {
        AlertView {
            key,
            alert_id: format!("a-{key}"),
            severity,
            ufs: ufs.to_vec(),
            headline: headline.into(),
            area: String::new(),
            source: "defesa-civil-idap".into(),
            sent: format!("2026-10-27T12:00:0{key}+00:00"),
            url: None,
        }
    }

    #[test]
    fn reads_a_record() {
        let r: Record = serde_json::from_value(json!({
            "id": 7, "alert_id": "x", "severity": "Severe", "ufs": ["RS", "SC"],
            "headline": "TESTE: h.", "area": "a", "source": "mock", "sent": "s", "url": null
        }))
        .unwrap();
        let a = AlertView::from_record(&r).unwrap();
        assert_eq!(a.severity, Severity::Severe);
        assert_eq!(a.ufs, vec![Uf::Rs, Uf::Sc]);
        assert!(a.is_test());
        let mut bad = r.clone();
        bad.insert("severity".into(), json!("Huge"));
        assert_eq!(AlertView::from_record(&bad), Err("severity".into()));
    }

    #[test]
    fn a_uf_takes_the_highest_severity() {
        let all = [
            alert(1, Severity::Minor, &[Uf::Pr, Uf::Sc], "a"),
            alert(2, Severity::Extreme, &[Uf::Sc], "b"),
        ];
        let m = by_uf(&all);
        assert_eq!(m[&Uf::Sc], (Severity::Extreme, 2));
        assert_eq!(m[&Uf::Pr], (Severity::Minor, 1));
        assert!(!m.contains_key(&Uf::Rs));
    }

    #[test]
    fn the_filter_and_the_order() {
        let all = [
            alert(1, Severity::Minor, &[Uf::Pr], "a"),
            alert(2, Severity::Extreme, &[Uf::Rs], "TESTE: b"),
            alert(3, Severity::Extreme, &[Uf::Sc], "c"),
        ];
        let keys = |f: &Filter| visible(&all, f).iter().map(|a| a.key).collect::<Vec<_>>();
        assert_eq!(keys(&Filter::default()), [3, 2, 1]);
        let mut f = Filter::default();
        f.toggle(Severity::Extreme);
        assert_eq!(keys(&f), [1]);
        f.toggle(Severity::Extreme);
        f.tests = false;
        assert_eq!(keys(&f), [3, 1]);
        f.uf = Some(Uf::Pr);
        assert_eq!(keys(&f), [1]);
        assert_eq!(keys(&f.without_uf()), [3, 1]);
    }

    #[test]
    fn ibge_borders_get_the_uf_and_the_bounds() {
        let raw = json!({"type": "FeatureCollection", "features": [
            {"type": "Feature", "properties": {"codarea": "43"},
             "geometry": {"type": "MultiPolygon", "coordinates": [[[[-57.6, -30.1], [-49.7, -27.1], [-53.0, -33.7], [-57.6, -30.1]]]]}},
            {"type": "Feature", "properties": {"codarea": "42"},
             "geometry": {"type": "Polygon", "coordinates": [[[-53.8, -26.0], [-48.3, -25.9], [-49.0, -29.3], [-53.8, -26.0]]]}}
        ]});
        let (geo, boxes) = ufs_geojson(raw).unwrap();
        assert_eq!(geo["features"][0]["properties"]["uf"], "RS");
        assert_eq!(boxes[&Uf::Rs], [-57.6, -33.7, -49.7, -27.1]);
        assert_eq!(
            union(&boxes, [Uf::Rs, Uf::Sc, Uf::Sp].iter()),
            Some([-57.6, -33.7, -48.3, -25.9])
        );
        let bad = json!({"features": [{"properties": {"codarea": "99"}, "geometry": null}]});
        assert!(ufs_geojson(bad).is_err());
    }
}
