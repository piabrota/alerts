//! `trilhas`: the Wikiloc trails raw-source, `data/raw/trilhas/`.
//!
//! ```console
//! cargo run -q -p smart-alerts-trilhas -- slug "Recreio dos Bandeirantes"   # recreio-dos-bandeirantes
//! cargo run -q -p smart-alerts-trilhas -- praias < nomes.tsv >> data/raw/trilhas/praias.tsv
//! cargo run -q -p smart-alerts-trilhas -- gpx rio joatinga 123456 trilha.gpx >> data/raw/trilhas/trilhas.tsv
//! ```
//!
//! `praias` reads `cidade<TAB>nome` lines and writes `cidade<TAB>slug<TAB>nome`.
//! `gpx` reads one GPX file exported from Wikiloc and writes one row of `trilhas.tsv`:
//! `cidade praia id km dmais lat lon nome`. `id` is the Wikiloc trail id, so the trail page is
//! `https://www.wikiloc.com/wikiloc/view.do?id=<id>`. `lat lon` is the first point of the track.
//! This tool never fetches anything: it reads files that a person exported.

use std::fs;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

/// The beach table that `gpx` checks the beach slug against.
const PRAIAS: &str = "data/raw/trilhas/praias.tsv";
/// The city codes: Salvador, Rio de Janeiro, Florianópolis.
const CIDADES: [&str; 3] = ["ssa", "rio", "fln"];
/// The mean Earth radius in kilometres (IUGG).
const R_KM: f64 = 6371.0088;

/// One track point: latitude, longitude, optional elevation in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Ponto {
    lat: f64,
    lon: f64,
    ele: Option<f64>,
}

/// One row of `trilhas.tsv`, without the city, beach and id.
#[derive(Debug, PartialEq)]
struct Trilha {
    nome: String,
    km: f64,
    dmais: f64,
    inicio: Ponto,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["slug", nome] => Ok(format!("{}\n", slug(nome))),
        ["praias"] => praias(io::stdin().lock()),
        ["gpx", cidade, praia, id, arquivo] => gpx(cidade, praia, id, arquivo),
        _ => Err(USO.to_string()),
    };
    match out {
        Ok(text) => match io::stdout().write_all(text.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("trilhas: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("trilhas: {e}");
            ExitCode::FAILURE
        }
    }
}

const USO: &str = "uso: trilhas slug <nome> | trilhas praias < cidade-nome.tsv | trilhas gpx <cidade> <praia> <id> <arquivo.gpx>";

/// A beach name as a slug: lowercase ASCII, words joined by `-`.
fn slug(nome: &str) -> String {
    let mut s = String::new();
    for c in nome.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

fn cidade_valida(cidade: &str) -> Result<(), String> {
    if CIDADES.contains(&cidade) {
        Ok(())
    } else {
        Err(format!(
            "cidade desconhecida: {cidade} (use {})",
            CIDADES.join(", ")
        ))
    }
}

fn praias(input: impl BufRead) -> Result<String, String> {
    let mut out = String::new();
    for linha in input.lines() {
        let linha = linha.map_err(|e| e.to_string())?;
        if linha.trim().is_empty() {
            continue;
        }
        let (cidade, nome) = linha
            .split_once('\t')
            .ok_or_else(|| format!("linha sem TAB: {linha}"))?;
        cidade_valida(cidade)?;
        out.push_str(&format!("{cidade}\t{}\t{}\n", slug(nome), nome.trim()));
    }
    Ok(out)
}

fn gpx(cidade: &str, praia: &str, id: &str, arquivo: &str) -> Result<String, String> {
    cidade_valida(cidade)?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("id do Wikiloc deve ser um número: {id}"));
    }
    let tabela = fs::read_to_string(PRAIAS).map_err(|e| format!("{PRAIAS}: {e}"))?;
    let existe = tabela.lines().skip(1).any(|l| {
        let mut c = l.split('\t');
        c.next() == Some(cidade) && c.next() == Some(praia)
    });
    if !existe {
        return Err(format!("praia {cidade}/{praia} não está em {PRAIAS}"));
    }
    let xml = fs::read_to_string(arquivo).map_err(|e| format!("{arquivo}: {e}"))?;
    let t = ler_gpx(&xml).ok_or_else(|| format!("{arquivo}: nenhum ponto de trilha"))?;
    Ok(format!(
        "{cidade}\t{praia}\t{id}\t{:.2}\t{:.0}\t{:.5}\t{:.5}\t{}\n",
        t.km, t.dmais, t.inicio.lat, t.inicio.lon, t.nome
    ))
}

/// Reads the name and the points of a GPX track (`trkpt`, or `rtept` for a route).
fn ler_gpx(xml: &str) -> Option<Trilha> {
    let mut pontos = Vec::new();
    for tag in ["<trkpt", "<rtept"] {
        let fim = if tag == "<trkpt" {
            "</trkpt>"
        } else {
            "</rtept>"
        };
        let mut resto = xml;
        while let Some(i) = resto.find(tag) {
            let bloco = &resto[i..];
            let abre = bloco.find('>')?;
            let cabeca = &bloco[..abre];
            let corpo_fim = if cabeca.ends_with('/') {
                abre
            } else {
                bloco.find(fim).unwrap_or(abre)
            };
            let corpo = &bloco[abre..corpo_fim];
            if let (Some(lat), Some(lon)) = (atributo(cabeca, "lat"), atributo(cabeca, "lon")) {
                let ele = entre(corpo, "<ele>", "</ele>").and_then(|e| e.trim().parse().ok());
                pontos.push(Ponto { lat, lon, ele });
            }
            resto = &bloco[corpo_fim.max(1)..];
        }
        if !pontos.is_empty() {
            break;
        }
    }
    let inicio = *pontos.first()?;
    let km = pontos.windows(2).map(|w| haversine(w[0], w[1])).sum();
    let dmais = pontos
        .windows(2)
        .filter_map(|w| Some(w[1].ele? - w[0].ele?))
        .filter(|d| *d > 0.0)
        .sum();
    let secao = xml.find("<trk>").map_or(xml, |i| &xml[i..]);
    let nome = entre(secao, "<name>", "</name>")
        .map(|n| {
            limpar(
                n.trim()
                    .trim_start_matches("<![CDATA[")
                    .trim_end_matches("]]>"),
            )
        })
        .unwrap_or_default();
    Some(Trilha {
        nome,
        km,
        dmais,
        inicio,
    })
}

fn atributo(cabeca: &str, nome: &str) -> Option<f64> {
    let chave = format!(" {nome}=");
    let i = cabeca.find(&chave)? + chave.len();
    let resto = &cabeca[i..];
    let aspas = resto.chars().next()?;
    let valor = resto[1..].split(aspas).next()?;
    valor.parse().ok()
}

fn entre<'a>(s: &'a str, abre: &str, fecha: &str) -> Option<&'a str> {
    let i = s.find(abre)? + abre.len();
    let j = s[i..].find(fecha)?;
    Some(&s[i..i + j])
}

/// Decodes the XML entities and keeps the name on one TSV cell.
fn limpar(nome: &str) -> String {
    nome.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The great-circle distance between two points, in kilometres.
fn haversine(a: Ponto, b: Ponto) -> f64 {
    let (la1, la2) = (a.lat.to_radians(), b.lat.to_radians());
    let dla = la2 - la1;
    let dlo = (b.lon - a.lon).to_radians();
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * R_KM * h.sqrt().asin()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn slug_tira_acento_e_espaco() {
        assert_eq!(slug("Recreio dos Bandeirantes"), "recreio-dos-bandeirantes");
        assert_eq!(slug("Piatã"), "piata");
        assert_eq!(slug("Moçambique"), "mocambique");
        assert_eq!(slug("  Pântano do Sul! "), "pantano-do-sul");
    }

    #[test]
    fn praias_gera_slug() {
        let out = praias("rio\tSão Conrado\nfln\tJoaquina\n".as_bytes()).unwrap();
        assert_eq!(
            out,
            "rio\tsao-conrado\tSão Conrado\nfln\tjoaquina\tJoaquina\n"
        );
        assert!(praias("poa\tGuaíba\n".as_bytes()).is_err());
    }

    #[test]
    fn le_gpx_do_wikiloc() {
        let xml = r#"<?xml version="1.0"?>
<gpx><metadata><name>Wikiloc</name></metadata>
<trk><name><![CDATA[Trilha da Joatinga &amp; Pedra]]></name><trkseg>
<trkpt lat="-23.01500" lon="-43.29000"><ele>10</ele></trkpt>
<trkpt lat="-23.01600" lon="-43.29000"><ele>25</ele></trkpt>
<trkpt lat='-23.01700' lon='-43.29000'><ele>20</ele></trkpt>
</trkseg></trk></gpx>"#;
        let t = ler_gpx(xml).unwrap();
        assert_eq!(t.nome, "Trilha da Joatinga & Pedra");
        assert!((t.km - 0.2224).abs() < 0.001, "km = {}", t.km);
        assert_eq!(t.dmais, 15.0);
        assert_eq!(t.inicio.lat, -23.015);
    }

    #[test]
    fn gpx_sem_pontos_falha() {
        assert!(ler_gpx("<gpx><trk><name>x</name></trk></gpx>").is_none());
    }
}
