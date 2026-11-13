-- | The curated catalog of official alert sources, in Brazil and in the world, and its search.
--
-- The catalog is data: one 'Entry' per endpoint of an authoritative agency. It says where the
-- alerts are, in which format, and whether the endpoint needs a token. A source that needs a
-- token is BYOT (bring your own token): alerts never ships a token, each person registers with
-- the agency and sets the token in the environment variable that 'Token' names.
--
-- The module does no I/O. Being in the catalog does not make an entry a source of the service:
-- 'entryModule' names the source module, when one exists.
module Alerts.Catalog
  ( Auth (..),
    Kind (..),
    Entry (..),
    tokenEnv,
    catalog,
    Query (..),
    emptyQuery,
    search,
    getEntry,
    byot,
    foldAccents,
  )
where

import Data.Char (toLower, toUpper)
import Data.List (find, isInfixOf)

-- | What an endpoint needs to answer.
data Auth
  = NoAuth
  | -- | BYOT: the environment variable that holds the token, and how a person gets it.
    Token String String
  deriving (Show, Eq)

-- | What the endpoint gives: warnings ready to deliver, or measurements that a harness would
-- turn into alerts.
data Kind = AlertsKind | Observations
  deriving (Show, Eq)

-- | One endpoint of an official agency.
data Entry = Entry
  { -- | A stable id, lowercase with @-@, for example @cemaden@.
    entryId :: String,
    entryName :: String,
    entryAgency :: String,
    -- | ISO 3166-1 alpha-2 codes. Empty means the whole world.
    entryCountries :: [String],
    entryHazards :: [String],
    entryKind :: Kind,
    entryFormat :: String,
    entryUrl :: String,
    entryDocs :: Maybe String,
    entryAuth :: Auth,
    -- | The id of the source module that reads this entry, when one exists.
    entryModule :: Maybe String,
    -- | What a person checked, and when. A claim that nobody checked carries @⚠@.
    entryChecked :: String
  }
  deriving (Show, Eq)

-- | The environment variable of a BYOT token: @ALERTS_TOKEN_@ and the id in uppercase, with
-- @_@ for @-@.
tokenEnv :: String -> String
tokenEnv i = "ALERTS_TOKEN_" <> map (\c -> if c == '-' then '_' else toUpper c) i

data Query = Query
  { -- | Words that must all appear in the entry, with no regard to case or accents.
    qText :: String,
    -- | An ISO 3166-1 alpha-2 code. A world source matches every country.
    qCountry :: Maybe String,
    qNoToken :: Bool,
    qAlertsOnly :: Bool
  }
  deriving (Show, Eq)

emptyQuery :: Query
emptyQuery = Query "" Nothing False False

-- | The entries that match the query, in catalog order.
search :: Query -> [Entry]
search q = filter ok catalog
  where
    ws = words (foldAccents (qText q))
    up = map toUpper
    trimmed = unwords . words
    ok e =
      maybe True (\c -> null (entryCountries e) || any ((== up (trimmed c)) . up) (entryCountries e)) (qCountry q)
        && (not (qNoToken q) || entryAuth e == NoAuth)
        && (not (qAlertsOnly q) || entryKind e == AlertsKind)
        && all (`isInfixOf` foldAccents (haystack e)) ws

getEntry :: String -> Maybe Entry
getEntry i = find ((== i) . entryId) catalog

-- | The entries that need a token, with the variable of each.
byot :: [(Entry, String)]
byot = [(e, v) | e <- catalog, Token v _ <- [entryAuth e]]

haystack :: Entry -> String
haystack e =
  unwords $
    [entryId e, entryName e, entryAgency e, entryFormat e]
      <> entryHazards e
      <> entryCountries e
      <> ["world global" | null (entryCountries e)]

-- | Lowercase, with the accents of Portuguese, Spanish and French removed.
foldAccents :: String -> String
foldAccents = map (plain . toLower)
  where
    plain c
      | c `elem` ("áàâãä" :: String) = 'a'
      | c `elem` ("éèêë" :: String) = 'e'
      | c `elem` ("íìîï" :: String) = 'i'
      | c `elem` ("óòôõö" :: String) = 'o'
      | c `elem` ("úùûü" :: String) = 'u'
      | c == 'ç' = 'c'
      | c == 'ñ' = 'n'
      | otherwise = c

-- | The catalog, Brazil first. The checks were made on 2026-10-28 from inside Brazil, with the
-- honest User-Agent of alerts.
catalog :: [Entry]
catalog =
  [ Entry
      { entryId = "cemaden",
        entryName = "Painel de alertas",
        entryAgency = "CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais)",
        entryCountries = ["BR"],
        entryHazards = ["flood", "landslide"],
        entryKind = AlertsKind,
        entryFormat = "JSON",
        entryUrl = "https://painelalertas.cemaden.gov.br/wsAlertas2",
        entryDocs = Nothing,
        entryAuth = NoAuth,
        entryModule = Just "cemaden",
        entryChecked = "2026-10-28: 200 with municipal alerts; undocumented ⚠"
      },
    Entry
      { entryId = "cemaden-ped",
        entryName = "PED: dados das PCDs",
        entryAgency = "CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais)",
        entryCountries = ["BR"],
        entryHazards = ["flood", "landslide"],
        entryKind = Observations,
        entryFormat = "JSON",
        entryUrl = "https://sws.cemaden.gov.br/PED/rest/pcds/pcds-dados-recentes",
        entryDocs = Just "https://sws.cemaden.gov.br/PED/api/",
        entryAuth = Token "ALERTS_TOKEN_CEMADEN_PED" "register for free on PED, then POST email and password to https://sgaa.cemaden.gov.br/SGAA/rest/controle-token/tokens; the JWT goes in the `token` header",
        entryModule = Nothing,
        entryChecked = "2026-10-28: without a token, 400 \"O parâmetro token não pode ser vazio!\""
      },
    Entry
      { entryId = "defesa-civil-idap",
        entryName = "IDAP: alertas públicos",
        entryAgency = "Defesa Civil Nacional (MIDR)",
        entryCountries = ["BR"],
        entryHazards = ["flood", "landslide", "storm", "drought", "fire"],
        entryKind = AlertsKind,
        entryFormat = "Atom + CAP 1.2",
        entryUrl = "https://idapfile.mdr.gov.br/idap/api/rss/cap",
        entryDocs = Nothing,
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200; keeps about 12 hours ⚠; undocumented ⚠"
      },
    Entry
      { entryId = "inmet",
        entryName = "Avisos meteorológicos",
        entryAgency = "INMET (Instituto Nacional de Meteorologia)",
        entryCountries = ["BR"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "drought"],
        entryKind = AlertsKind,
        entryFormat = "RSS + CAP 1.2",
        entryUrl = "https://apiprevmet3.inmet.gov.br/avisos/rss",
        entryDocs = Just "https://portal.inmet.gov.br/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200 with the honest User-Agent"
      },
    Entry
      { entryId = "inmet-estacoes",
        entryName = "Dados das estações",
        entryAgency = "INMET (Instituto Nacional de Meteorologia)",
        entryCountries = ["BR"],
        entryHazards = ["rain", "heat", "cold", "wind"],
        entryKind = Observations,
        entryFormat = "JSON",
        entryUrl = "https://apitempo.inmet.gov.br/token/estacao/{start}/{end}/{station}/{token}",
        entryDocs = Nothing,
        entryAuth = Token "ALERTS_TOKEN_INMET_ESTACOES" "ask INMET for a free token by email",
        entryModule = Nothing,
        entryChecked = "not checked with a token ⚠; undocumented ⚠"
      },
    Entry
      { entryId = "ana",
        entryName = "Hidroweb: estações telemétricas",
        entryAgency = "ANA (Agência Nacional de Águas e Saneamento Básico)",
        entryCountries = ["BR"],
        entryHazards = ["flood", "drought"],
        entryKind = Observations,
        entryFormat = "JSON",
        entryUrl = "https://www.ana.gov.br/hidrowebservice/EstacoesTelemetricas/",
        entryDocs = Just "https://www.ana.gov.br/hidrowebservice/swagger-ui/index.html",
        entryAuth = Token "ALERTS_TOKEN_ANA" "register by email with a CPF or CNPJ; the token is valid for 60 minutes",
        entryModule = Nothing,
        entryChecked = "not checked with a token ⚠"
      },
    Entry
      { entryId = "smn-ar",
        entryName = "Avisos a corto plazo",
        entryAgency = "SMN Argentina (Servicio Meteorológico Nacional)",
        entryCountries = ["AR"],
        entryHazards = ["storm", "rain", "wind"],
        entryKind = AlertsKind,
        entryFormat = "GeoRSS",
        entryUrl = "https://ssl.smn.gob.ar/feeds/avisocorto_GeoRSS.xml",
        entryDocs = Nothing,
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200"
      },
    Entry
      { entryId = "nws",
        entryName = "Active alerts",
        entryAgency = "NWS (US National Weather Service, NOAA)",
        entryCountries = ["US"],
        entryHazards = ["flood", "storm", "tornado", "hurricane", "heat", "cold", "fire", "tsunami"],
        entryKind = AlertsKind,
        entryFormat = "GeoJSON + CAP 1.2",
        entryUrl = "https://api.weather.gov/alerts/active",
        entryDocs = Just "https://www.weather.gov/documentation/services-web-api",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200; NWS asks for a User-Agent with a contact"
      },
    Entry
      { entryId = "eccc",
        entryName = "Alertes météo (Datamart)",
        entryAgency = "ECCC (Environment and Climate Change Canada)",
        entryCountries = ["CA"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "flood"],
        entryKind = AlertsKind,
        entryFormat = "CAP 1.2 files",
        entryUrl = "https://dd.weather.gc.ca/",
        entryDocs = Just "https://eccc-msc.github.io/open-data/msc-datamart/readme_en/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: the Datamart answers; the path of the CAP files ⚠"
      },
    Entry
      { entryId = "meteoalarm",
        entryName = "MeteoAlarm feeds",
        entryAgency = "EUMETNET (the national weather services of Europe)",
        entryCountries = ["AT", "BA", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GB", "GR", "HR", "HU", "IE", "IL", "IS", "IT", "LT", "LU", "LV", "MD", "ME", "MK", "MT", "NL", "NO", "PL", "PT", "RO", "RS", "SE", "SI", "SK", "UA"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "flood", "fire", "snow"],
        entryKind = AlertsKind,
        entryFormat = "Atom + CAP 1.2",
        entryUrl = "https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-{country}",
        entryDocs = Just "https://feeds.meteoalarm.org/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200 for `portugal`; the list of member countries ⚠"
      },
    Entry
      { entryId = "dwd",
        entryName = "Warnungen",
        entryAgency = "DWD (Deutscher Wetterdienst)",
        entryCountries = ["DE"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "snow"],
        entryKind = AlertsKind,
        entryFormat = "JSONP",
        entryUrl = "https://www.dwd.de/DWD/warnungen/warnapp/json/warnings.json",
        entryDocs = Just "https://opendata.dwd.de/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200"
      },
    Entry
      { entryId = "meteo-france",
        entryName = "Vigilance météorologique",
        entryAgency = "Météo-France",
        entryCountries = ["FR"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "flood", "snow", "avalanche"],
        entryKind = AlertsKind,
        entryFormat = "JSON",
        entryUrl = "https://public-api.meteofrance.fr/public/DPVigilance/v1/cartevigilance/encours",
        entryDocs = Just "https://portail-api.meteofrance.fr/",
        entryAuth = Token "ALERTS_TOKEN_METEO_FRANCE" "create a free account on portail-api.meteofrance.fr and subscribe to the DonneesPubliquesVigilance API; the key goes in the `apikey` header",
        entryModule = Nothing,
        entryChecked = "2026-10-28: without a token, 401 \"Missing Credentials\""
      },
    Entry
      { entryId = "aemet",
        entryName = "Avisos CAP (Meteoalerta)",
        entryAgency = "AEMET (Agencia Estatal de Meteorología)",
        entryCountries = ["ES"],
        entryHazards = ["storm", "rain", "wind", "heat", "cold", "snow", "coastal"],
        entryKind = AlertsKind,
        entryFormat = "CAP 1.2 (tar)",
        entryUrl = "https://opendata.aemet.es/opendata/api/avisos_cap/ultimoelaborado/area/{area}",
        entryDocs = Just "https://opendata.aemet.es/",
        entryAuth = Token "ALERTS_TOKEN_AEMET" "ask for a free API key by email at opendata.aemet.es; the key goes in the `api_key` header",
        entryModule = Nothing,
        entryChecked = "2026-10-28: no answer in 25 s ⚠"
      },
    Entry
      { entryId = "jma",
        entryName = "防災情報XML (extra feed)",
        entryAgency = "JMA (Japan Meteorological Agency)",
        entryCountries = ["JP"],
        entryHazards = ["storm", "rain", "flood", "landslide", "tsunami", "earthquake", "volcano"],
        entryKind = AlertsKind,
        entryFormat = "Atom + JMA XML",
        entryUrl = "https://www.data.jma.go.jp/developer/xml/feed/extra.xml",
        entryDocs = Just "https://xml.kishou.go.jp/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200"
      },
    Entry
      { entryId = "gdacs",
        entryName = "Global Disaster Alert and Coordination System",
        entryAgency = "GDACS (United Nations and the European Commission)",
        entryCountries = [],
        entryHazards = ["earthquake", "tsunami", "cyclone", "flood", "volcano", "drought", "fire"],
        entryKind = AlertsKind,
        entryFormat = "RSS",
        entryUrl = "https://www.gdacs.org/xml/rss.xml",
        entryDocs = Just "https://www.gdacs.org/",
        entryAuth = NoAuth,
        entryModule = Nothing,
        entryChecked = "2026-10-28: 200"
      }
  ]
