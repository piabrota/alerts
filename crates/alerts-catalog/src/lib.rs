//! The curated catalog of official alert sources, in Brazil and in the world, and its search.
//!
//! The catalog is data: one [`Entry`] per endpoint of an authoritative agency. It says where the
//! alerts are, in which format, and whether the endpoint needs a token. A source that needs a
//! token is BYOT (bring your own token): alerts never ships a token, each person registers with
//! the agency and sets the token in the environment variable that [`Auth::Token`] names.
//!
//! The crate does no I/O. Being in the catalog does not make an entry a source of the service:
//! [`Entry::module`] names the source module, when one exists.

/// What an endpoint needs to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    None,
    /// BYOT: the person brings the token of their own account with the agency.
    Token {
        /// The environment variable (or `.env` line) that holds the token.
        env: &'static str,
        /// How a person gets the token.
        how: &'static str,
    },
}

/// What the endpoint gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Warnings or alerts, ready to deliver.
    Alerts,
    /// Measurements (rain, river levels): a harness would turn them into alerts.
    Observations,
}

/// One endpoint of an official agency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// A stable id, lowercase with `-`, for example `cemaden`.
    pub id: &'static str,
    pub name: &'static str,
    pub agency: &'static str,
    /// ISO 3166-1 alpha-2 codes of the countries the source covers. Empty means the whole world.
    pub countries: &'static [&'static str],
    /// The hazards, in English, for example `flood`, `landslide`, `storm`.
    pub hazards: &'static [&'static str],
    pub kind: Kind,
    pub format: &'static str,
    pub url: &'static str,
    /// The agency's documentation of the endpoint, when it has one.
    pub docs: Option<&'static str>,
    pub auth: Auth,
    /// The id of the alerts source module that reads this entry, when one exists.
    pub module: Option<&'static str>,
    /// What a person checked, and when. A claim that nobody checked carries `⚠`.
    pub checked: &'static str,
}

/// The environment variable of a BYOT token: `ALERTS_TOKEN_` and the id in uppercase, with `_`
/// for `-`.
pub fn token_env(id: &str) -> String {
    format!("ALERTS_TOKEN_{}", id.to_ascii_uppercase().replace('-', "_"))
}

const BR: &[&str] = &["BR"];

/// The catalog, Brazil first. The checks were made on 2026-10-28 from inside Brazil, with the
/// honest User-Agent of alerts.
pub const CATALOG: &[Entry] = &[
    Entry {
        id: "cemaden",
        name: "Painel de alertas",
        agency: "CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais)",
        countries: BR,
        hazards: &["flood", "landslide"],
        kind: Kind::Alerts,
        format: "JSON",
        url: "https://painelalertas.cemaden.gov.br/wsAlertas2",
        docs: None,
        auth: Auth::None,
        module: Some("cemaden"),
        checked: "2026-10-28: 200 with municipal alerts; undocumented ⚠",
    },
    Entry {
        id: "cemaden-ped",
        name: "PED: dados das PCDs",
        agency: "CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais)",
        countries: BR,
        hazards: &["flood", "landslide"],
        kind: Kind::Observations,
        format: "JSON",
        url: "https://sws.cemaden.gov.br/PED/rest/pcds/pcds-dados-recentes",
        docs: Some("https://sws.cemaden.gov.br/PED/api/"),
        auth: Auth::Token {
            env: "ALERTS_TOKEN_CEMADEN_PED",
            how: "register for free on PED, then POST email and password to \
                  https://sgaa.cemaden.gov.br/SGAA/rest/controle-token/tokens; the JWT goes in the `token` header",
        },
        module: None,
        checked: "2026-10-28: without a token, 400 \"O parâmetro token não pode ser vazio!\"",
    },
    Entry {
        id: "defesa-civil-idap",
        name: "IDAP: alertas públicos",
        agency: "Defesa Civil Nacional (MIDR)",
        countries: BR,
        hazards: &["flood", "landslide", "storm", "drought", "fire"],
        kind: Kind::Alerts,
        format: "Atom + CAP 1.2",
        url: "https://idapfile.mdr.gov.br/idap/api/rss/cap",
        docs: None,
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200; keeps about 12 hours ⚠; undocumented ⚠",
    },
    Entry {
        id: "inmet",
        name: "Avisos meteorológicos",
        agency: "INMET (Instituto Nacional de Meteorologia)",
        countries: BR,
        hazards: &["storm", "rain", "wind", "heat", "cold", "drought"],
        kind: Kind::Alerts,
        format: "RSS + CAP 1.2",
        url: "https://apiprevmet3.inmet.gov.br/avisos/rss",
        docs: Some("https://portal.inmet.gov.br/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200 with the honest User-Agent",
    },
    Entry {
        id: "inmet-estacoes",
        name: "Dados das estações",
        agency: "INMET (Instituto Nacional de Meteorologia)",
        countries: BR,
        hazards: &["rain", "heat", "cold", "wind"],
        kind: Kind::Observations,
        format: "JSON",
        url: "https://apitempo.inmet.gov.br/token/estacao/{start}/{end}/{station}/{token}",
        docs: None,
        auth: Auth::Token {
            env: "ALERTS_TOKEN_INMET_ESTACOES",
            how: "ask INMET for a free token by email",
        },
        module: None,
        checked: "not checked with a token ⚠; undocumented ⚠",
    },
    Entry {
        id: "ana",
        name: "Hidroweb: estações telemétricas",
        agency: "ANA (Agência Nacional de Águas e Saneamento Básico)",
        countries: BR,
        hazards: &["flood", "drought"],
        kind: Kind::Observations,
        format: "JSON",
        url: "https://www.ana.gov.br/hidrowebservice/EstacoesTelemetricas/",
        docs: Some("https://www.ana.gov.br/hidrowebservice/swagger-ui/index.html"),
        auth: Auth::Token {
            env: "ALERTS_TOKEN_ANA",
            how: "register by email with a CPF or CNPJ; the token is valid for 60 minutes",
        },
        module: None,
        checked: "not checked with a token ⚠",
    },
    Entry {
        id: "smn-ar",
        name: "Avisos a corto plazo",
        agency: "SMN Argentina (Servicio Meteorológico Nacional)",
        countries: &["AR"],
        hazards: &["storm", "rain", "wind"],
        kind: Kind::Alerts,
        format: "GeoRSS",
        url: "https://ssl.smn.gob.ar/feeds/avisocorto_GeoRSS.xml",
        docs: None,
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200",
    },
    Entry {
        id: "nws",
        name: "Active alerts",
        agency: "NWS (US National Weather Service, NOAA)",
        countries: &["US"],
        hazards: &[
            "flood",
            "storm",
            "tornado",
            "hurricane",
            "heat",
            "cold",
            "fire",
            "tsunami",
        ],
        kind: Kind::Alerts,
        format: "GeoJSON + CAP 1.2",
        url: "https://api.weather.gov/alerts/active",
        docs: Some("https://www.weather.gov/documentation/services-web-api"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200; NWS asks for a User-Agent with a contact",
    },
    Entry {
        id: "eccc",
        name: "Alertes météo (Datamart)",
        agency: "ECCC (Environment and Climate Change Canada)",
        countries: &["CA"],
        hazards: &["storm", "rain", "wind", "heat", "cold", "flood"],
        kind: Kind::Alerts,
        format: "CAP 1.2 files",
        url: "https://dd.weather.gc.ca/",
        docs: Some("https://eccc-msc.github.io/open-data/msc-datamart/readme_en/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: the Datamart answers; the path of the CAP files ⚠",
    },
    Entry {
        id: "meteoalarm",
        name: "MeteoAlarm feeds",
        agency: "EUMETNET (the national weather services of Europe)",
        countries: &[
            "AT", "BA", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GB",
            "GR", "HR", "HU", "IE", "IL", "IS", "IT", "LT", "LU", "LV", "MD", "ME", "MK", "MT",
            "NL", "NO", "PL", "PT", "RO", "RS", "SE", "SI", "SK", "UA",
        ],
        hazards: &[
            "storm", "rain", "wind", "heat", "cold", "flood", "fire", "snow",
        ],
        kind: Kind::Alerts,
        format: "Atom + CAP 1.2",
        url: "https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-{country}",
        docs: Some("https://feeds.meteoalarm.org/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200 for `portugal`; the list of member countries ⚠",
    },
    Entry {
        id: "dwd",
        name: "Warnungen",
        agency: "DWD (Deutscher Wetterdienst)",
        countries: &["DE"],
        hazards: &["storm", "rain", "wind", "heat", "cold", "snow"],
        kind: Kind::Alerts,
        format: "JSONP",
        url: "https://www.dwd.de/DWD/warnungen/warnapp/json/warnings.json",
        docs: Some("https://opendata.dwd.de/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200",
    },
    Entry {
        id: "meteo-france",
        name: "Vigilance météorologique",
        agency: "Météo-France",
        countries: &["FR"],
        hazards: &[
            "storm",
            "rain",
            "wind",
            "heat",
            "cold",
            "flood",
            "snow",
            "avalanche",
        ],
        kind: Kind::Alerts,
        format: "JSON",
        url: "https://public-api.meteofrance.fr/public/DPVigilance/v1/cartevigilance/encours",
        docs: Some("https://portail-api.meteofrance.fr/"),
        auth: Auth::Token {
            env: "ALERTS_TOKEN_METEO_FRANCE",
            how: "create a free account on portail-api.meteofrance.fr and subscribe to the \
                  DonneesPubliquesVigilance API; the key goes in the `apikey` header",
        },
        module: None,
        checked: "2026-10-28: without a token, 401 \"Missing Credentials\"",
    },
    Entry {
        id: "aemet",
        name: "Avisos CAP (Meteoalerta)",
        agency: "AEMET (Agencia Estatal de Meteorología)",
        countries: &["ES"],
        hazards: &["storm", "rain", "wind", "heat", "cold", "snow", "coastal"],
        kind: Kind::Alerts,
        format: "CAP 1.2 (tar)",
        url: "https://opendata.aemet.es/opendata/api/avisos_cap/ultimoelaborado/area/{area}",
        docs: Some("https://opendata.aemet.es/"),
        auth: Auth::Token {
            env: "ALERTS_TOKEN_AEMET",
            how: "ask for a free API key by email at opendata.aemet.es; the key goes in the `api_key` header",
        },
        module: None,
        checked: "2026-10-28: no answer in 25 s ⚠",
    },
    Entry {
        id: "jma",
        name: "防災情報XML (extra feed)",
        agency: "JMA (Japan Meteorological Agency)",
        countries: &["JP"],
        hazards: &[
            "storm",
            "rain",
            "flood",
            "landslide",
            "tsunami",
            "earthquake",
            "volcano",
        ],
        kind: Kind::Alerts,
        format: "Atom + JMA XML",
        url: "https://www.data.jma.go.jp/developer/xml/feed/extra.xml",
        docs: Some("https://xml.kishou.go.jp/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200",
    },
    Entry {
        id: "gdacs",
        name: "Global Disaster Alert and Coordination System",
        agency: "GDACS (United Nations and the European Commission)",
        countries: &[],
        hazards: &[
            "earthquake",
            "tsunami",
            "cyclone",
            "flood",
            "volcano",
            "drought",
            "fire",
        ],
        kind: Kind::Alerts,
        format: "RSS",
        url: "https://www.gdacs.org/xml/rss.xml",
        docs: Some("https://www.gdacs.org/"),
        auth: Auth::None,
        module: None,
        checked: "2026-10-28: 200",
    },
];

/// A search of the catalog. Every part is optional; the empty query finds every entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Words that must all appear in the entry (id, name, agency, hazards, format, countries),
    /// with no regard to case or accents.
    pub text: String,
    /// An ISO 3166-1 alpha-2 code. A world source matches every country.
    pub country: Option<String>,
    /// Only the entries that need no token.
    pub no_token: bool,
    /// Only the entries that give alerts, not observations.
    pub alerts_only: bool,
}

/// The entries that match the query, in catalog order.
pub fn search(q: &Query) -> Vec<&'static Entry> {
    let words: Vec<String> = fold(&q.text)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    CATALOG
        .iter()
        .filter(|e| {
            q.country.as_deref().is_none_or(|c| {
                e.countries.is_empty()
                    || e.countries
                        .iter()
                        .any(|ec| ec.eq_ignore_ascii_case(c.trim()))
            })
        })
        .filter(|e| !q.no_token || e.auth == Auth::None)
        .filter(|e| !q.alerts_only || e.kind == Kind::Alerts)
        .filter(|e| {
            let hay = fold(&haystack(e));
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

/// The entry with this id.
pub fn get(id: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.id == id)
}

/// The entries that need a token.
pub fn byot() -> impl Iterator<Item = (&'static Entry, &'static str)> {
    CATALOG.iter().filter_map(|e| match e.auth {
        Auth::Token { env, .. } => Some((e, env)),
        Auth::None => None,
    })
}

fn haystack(e: &Entry) -> String {
    let mut hay = vec![e.id, e.name, e.agency, e.format];
    hay.extend(e.hazards);
    hay.extend(e.countries);
    if e.countries.is_empty() {
        hay.push("world global");
    }
    hay.join(" ")
}

/// Lowercase, with the accents of Portuguese, Spanish and French removed.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(q: Query) -> Vec<&'static str> {
        search(&q).iter().map(|e| e.id).collect()
    }

    #[test]
    fn ids_are_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for e in CATALOG {
            assert!(seen.insert(e.id), "duplicate id {}", e.id);
            assert!(
                e.id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            );
            assert!(e.url.starts_with("https://"), "{}", e.id);
            assert!(
                e.countries
                    .iter()
                    .all(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase()))
            );
            assert!(!e.hazards.is_empty() && !e.checked.is_empty());
        }
    }

    #[test]
    fn every_token_variable_follows_the_rule() {
        for (e, env) in byot() {
            assert_eq!(env, token_env(e.id));
        }
        assert_eq!(token_env("meteo-france"), "ALERTS_TOKEN_METEO_FRANCE");
    }

    #[test]
    fn empty_query_finds_everything() {
        assert_eq!(search(&Query::default()).len(), CATALOG.len());
    }

    #[test]
    fn search_by_text_ignores_case_and_accents() {
        assert_eq!(
            ids(Query {
                text: "CEMADEN painel".into(),
                ..Query::default()
            }),
            ["cemaden"]
        );
        assert!(
            ids(Query {
                text: "meteo france".into(),
                ..Query::default()
            })
            .contains(&"meteo-france")
        );
        assert!(
            ids(Query {
                text: "météorologique".into(),
                ..Query::default()
            })
            .contains(&"meteo-france")
        );
        assert!(
            ids(Query {
                text: "no such thing".into(),
                ..Query::default()
            })
            .is_empty()
        );
    }

    #[test]
    fn search_by_country_keeps_world_sources() {
        let br = ids(Query {
            country: Some("br".into()),
            ..Query::default()
        });
        assert!(br.contains(&"cemaden") && br.contains(&"gdacs") && !br.contains(&"nws"));
        let pt = ids(Query {
            country: Some("PT".into()),
            ..Query::default()
        });
        assert_eq!(pt, ["meteoalarm", "gdacs"]);
    }

    #[test]
    fn filters() {
        let free = search(&Query {
            no_token: true,
            ..Query::default()
        });
        assert!(free.iter().all(|e| e.auth == Auth::None));
        let br_alerts = ids(Query {
            country: Some("BR".into()),
            alerts_only: true,
            no_token: true,
            ..Query::default()
        });
        assert_eq!(
            br_alerts,
            ["cemaden", "defesa-civil-idap", "inmet", "gdacs"]
        );
    }

    #[test]
    fn modules_name_real_entries() {
        assert_eq!(get("cemaden").and_then(|e| e.module), Some("cemaden"));
        assert!(get("nope").is_none());
    }
}
