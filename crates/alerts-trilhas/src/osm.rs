//! The OpenStreetMap trails: an Overpass query per city, the Overpass JSON as rows of
//! `data/raw/trilhas/osm.tsv`, and those rows as GeoJSON for the map.
//!
//! A row is `cidade praia osm km nome linha`. `osm` is `w<id>` (a way) or `r<id>` (a relation),
//! so the page is `https://www.openstreetmap.org/way/<id>` or `/relation/<id>`. `praia` is the
//! slug of the nearest named OSM beach within [`PRAIA_KM`] of an end of the trail, or `-`.
//! `linha` is the geometry as encoded polylines (precision 5), one per OSM way, separated by a
//! space. The data is © OpenStreetMap contributors, ODbL 1.0.

use serde_json::{Value, json};

/// A trail end farther than this from every beach gets no beach.
const PRAIA_KM: f64 = 3.0;
/// The polyline precision: 5 decimals, about 1 m.
const FATOR: f64 = 1e5;

/// The bounding box of each city, as Overpass writes it: south, west, north, east.
pub const BBOX: [(&str, [f64; 4]); 3] = [
    ("ssa", [-13.02, -38.56, -12.85, -38.30]),
    ("rio", [-23.09, -43.80, -22.89, -43.10]),
    ("fln", [-27.85, -48.62, -27.38, -48.35]),
];

/// One point, as `(lat, lon)`.
type Pt = (f64, f64);

/// One trail: its OSM id, name and lines (one per OSM way).
#[derive(Debug, PartialEq)]
struct Trilha {
    osm: String,
    nome: String,
    linhas: Vec<Vec<Pt>>,
}

/// The Overpass QL for one city: named paths, hiking routes and named beaches, with geometry.
pub fn query(cidade: &str) -> Result<String, String> {
    let [s, w, n, e] = bbox(cidade)?;
    let b = format!("{s},{w},{n},{e}");
    Ok(format!(
        "[out:json][timeout:120];(way[\"highway\"=\"path\"][\"name\"]({b});relation[\"route\"=\"hiking\"]({b});way[\"natural\"=\"beach\"][\"name\"]({b}););out geom;"
    ))
}

fn bbox(cidade: &str) -> Result<[f64; 4], String> {
    BBOX.iter()
        .find(|(c, _)| *c == cidade)
        .map(|(_, b)| *b)
        .ok_or_else(|| format!("cidade desconhecida: {cidade} (use ssa, rio, fln)"))
}

/// The Overpass JSON of [`query`] as rows of `osm.tsv`, sorted by OSM id.
pub fn tsv(cidade: &str, overpass: &str) -> Result<String, String> {
    bbox(cidade)?;
    let v: Value = serde_json::from_str(overpass).map_err(|e| format!("overpass: {e}"))?;
    let elementos = v["elements"]
        .as_array()
        .ok_or("overpass: sem \"elements\"")?;
    let mut praias = Vec::new();
    let mut trilhas = Vec::new();
    for el in elementos {
        let tags = &el["tags"];
        let Some(nome) = tags["name"].as_str().map(super::limpar) else {
            continue;
        };
        let id = el["id"].as_u64().unwrap_or(0);
        match el["type"].as_str() {
            Some("way") if tags["natural"] == "beach" => {
                if let Some(c) = centro(&geometria(&el["geometry"])) {
                    praias.push((super::slug(&nome), c));
                }
            }
            Some("way") => trilhas.push(Trilha {
                osm: format!("w{id}"),
                nome,
                linhas: vec![geometria(&el["geometry"])],
            }),
            Some("relation") => trilhas.push(Trilha {
                osm: format!("r{id}"),
                nome,
                linhas: el["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|m| m["type"] == "way")
                    .map(|m| geometria(&m["geometry"]))
                    .collect(),
            }),
            _ => {}
        }
    }
    trilhas.retain(|t| t.linhas.iter().any(|l| l.len() >= 2));
    trilhas.sort_by(|a, b| a.osm.cmp(&b.osm));
    trilhas.dedup_by(|a, b| a.osm == b.osm);
    let mut out = String::new();
    for t in &trilhas {
        let km: f64 = t
            .linhas
            .iter()
            .flat_map(|l| l.windows(2))
            .map(|w| km(w[0], w[1]))
            .sum();
        let linha = t
            .linhas
            .iter()
            .filter(|l| l.len() >= 2)
            .map(|l| polyline(l))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "{cidade}\t{}\t{}\t{km:.2}\t{}\t{linha}\n",
            praia(t, &praias),
            t.osm,
            t.nome
        ));
    }
    Ok(out)
}

fn geometria(g: &Value) -> Vec<Pt> {
    g.as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p["lat"].as_f64()?, p["lon"].as_f64()?)))
        .collect()
}

fn centro(pts: &[Pt]) -> Option<Pt> {
    let n = pts.len() as f64;
    (!pts.is_empty()).then(|| {
        let (la, lo) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
        (la / n, lo / n)
    })
}

/// The slug of the beach nearest to an end of the trail, or `-`.
fn praia(t: &Trilha, praias: &[(String, Pt)]) -> String {
    let pontas: Vec<Pt> = t
        .linhas
        .iter()
        .flat_map(|l| [l.first(), l.last()])
        .flatten()
        .copied()
        .collect();
    praias
        .iter()
        .filter_map(|(slug, c)| {
            let d = pontas
                .iter()
                .map(|p| km(*p, *c))
                .fold(f64::INFINITY, f64::min);
            (d <= PRAIA_KM).then_some((d, slug))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or_else(|| "-".to_string(), |(_, s)| s.clone())
}

fn km(a: Pt, b: Pt) -> f64 {
    super::haversine(
        super::Ponto {
            lat: a.0,
            lon: a.1,
            ele: None,
        },
        super::Ponto {
            lat: b.0,
            lon: b.1,
            ele: None,
        },
    )
}

/// The encoded polyline algorithm (precision 5): `lat,lon` deltas as base-64 text.
fn polyline(pts: &[Pt]) -> String {
    let mut out = String::new();
    let mut prev = (0i64, 0i64);
    for &(lat, lon) in pts {
        let cur = ((lat * FATOR).round() as i64, (lon * FATOR).round() as i64);
        for d in [cur.0 - prev.0, cur.1 - prev.1] {
            let mut v = if d < 0 { !(d << 1) } else { d << 1 };
            while v >= 0x20 {
                out.push(char::from((((v & 0x1f) | 0x20) + 63) as u8));
                v >>= 5;
            }
            out.push(char::from((v + 63) as u8));
        }
        prev = cur;
    }
    out
}

fn decode(s: &str) -> Result<Vec<Pt>, String> {
    let mut pts = Vec::new();
    let (mut lat, mut lon) = (0i64, 0i64);
    let mut bytes = s.bytes();
    let mut next = || -> Result<Option<i64>, String> {
        let (mut v, mut shift) = (0i64, 0);
        loop {
            let Some(b) = bytes.next() else {
                return if shift == 0 {
                    Ok(None)
                } else {
                    Err(format!("polyline cortada: {s}"))
                };
            };
            let c = i64::from(b) - 63;
            if !(0..64).contains(&c) {
                return Err(format!("polyline inválida: {s}"));
            }
            v |= (c & 0x1f) << shift;
            shift += 5;
            if c < 0x20 {
                return Ok(Some(if v & 1 == 1 { !(v >> 1) } else { v >> 1 }));
            }
        }
    };
    while let Some(dlat) = next()? {
        let dlon = next()?.ok_or_else(|| format!("polyline cortada: {s}"))?;
        lat += dlat;
        lon += dlon;
        pts.push((lat as f64 / FATOR, lon as f64 / FATOR));
    }
    Ok(pts)
}

/// The rows of `osm.tsv` as a GeoJSON FeatureCollection: one MultiLineString per trail, and one
/// Point at its start (`"inicio": true`), so the map can mark trails at low zoom.
pub fn geojson(tsv: &str) -> Result<String, String> {
    let mut features = Vec::new();
    for (n, linha) in tsv.lines().enumerate() {
        if linha.trim().is_empty() || linha.starts_with("cidade\t") {
            continue;
        }
        let c: Vec<&str> = linha.split('\t').collect();
        let [cidade, praia, osm, km, nome, geo] = c.as_slice() else {
            return Err(format!("linha {}: esperava 6 colunas", n + 1));
        };
        let linhas = geo.split(' ').map(decode).collect::<Result<Vec<_>, _>>()?;
        let coords: Vec<Vec<[f64; 2]>> = linhas
            .iter()
            .map(|l| l.iter().map(|p| [p.1, p.0]).collect())
            .collect();
        let props = json!({ "cidade": cidade, "praia": praia, "osm": osm, "km": km.parse::<f64>().unwrap_or(0.0), "nome": nome });
        features.push(json!({ "type": "Feature", "properties": props,
            "geometry": { "type": "MultiLineString", "coordinates": coords } }));
        if let Some(p) = coords.first().and_then(|l| l.first()) {
            let mut props = props.clone();
            props["inicio"] = json!(true);
            features.push(json!({ "type": "Feature", "properties": props,
                "geometry": { "type": "Point", "coordinates": p } }));
        }
    }
    serde_json::to_string(&json!({ "type": "FeatureCollection", "features": features }))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn polyline_do_exemplo_do_google() {
        let pts = [(38.5, -120.2), (40.7, -120.95), (43.252, -126.453)];
        assert_eq!(polyline(&pts), "_p~iF~ps|U_ulLnnqC_mqNvxq`@");
        assert_eq!(decode("_p~iF~ps|U_ulLnnqC_mqNvxq`@").unwrap(), pts);
    }

    #[test]
    fn query_usa_o_bbox_da_cidade() {
        let q = query("rio").unwrap();
        assert!(q.contains("(-23.09,-43.8,-22.89,-43.1)"), "{q}");
        assert!(query("poa").is_err());
    }

    const OVERPASS: &str = r#"{"elements":[
      {"type":"way","id":10,"tags":{"natural":"beach","name":"Praia da Joatinga"},
       "geometry":[{"lat":-23.0150,"lon":-43.2900},{"lat":-23.0160,"lon":-43.2910}]},
      {"type":"way","id":7,"tags":{"highway":"path","name":"Trilha da Joatinga"},
       "geometry":[{"lat":-23.0140,"lon":-43.2880},{"lat":-23.0120,"lon":-43.2860}]},
      {"type":"relation","id":3,"tags":{"route":"hiking","name":"Transcarioca"},
       "members":[{"type":"way","geometry":[{"lat":-22.95,"lon":-43.40},{"lat":-22.96,"lon":-43.41}]},
                  {"type":"node","lat":-22.95,"lon":-43.40}]},
      {"type":"way","id":8,"tags":{"highway":"path"},"geometry":[{"lat":-23.0,"lon":-43.0},{"lat":-23.1,"lon":-43.1}]}
    ]}"#;

    #[test]
    fn tsv_liga_trilha_a_praia_mais_proxima() {
        let out = tsv("rio", OVERPASS).unwrap();
        let linhas: Vec<&str> = out.lines().collect();
        assert_eq!(linhas.len(), 2, "{out}");
        assert!(
            linhas[0].starts_with("rio\t-\tr3\t1.51\tTranscarioca\t"),
            "{out}"
        );
        assert!(
            linhas[1].starts_with("rio\tpraia-da-joatinga\tw7\t0.30\tTrilha da Joatinga\t"),
            "{out}"
        );
    }

    #[test]
    fn geojson_de_volta_com_ponto_inicial() {
        let g: Value =
            serde_json::from_str(&geojson(&tsv("rio", OVERPASS).unwrap()).unwrap()).unwrap();
        let f = g["features"].as_array().unwrap();
        assert_eq!(f.len(), 4);
        assert_eq!(f[2]["properties"]["osm"], "w7");
        assert_eq!(
            f[2]["geometry"]["coordinates"][0][0],
            json!([-43.288, -23.014])
        );
        assert_eq!(f[3]["geometry"]["type"], "Point");
        assert_eq!(f[3]["properties"]["inicio"], true);
    }
}
