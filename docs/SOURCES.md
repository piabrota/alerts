# Sources: endpoints and formats

This page lists the concrete endpoint, the format and the authentication of each source.
The README has the summary table.

Checked on 2026-10-22, from inside Brazil (`curl`) and from outside Brazil (an HTTP client on a
server abroad). "Official" means that the agency documents or links the endpoint. "Undocumented"
means that the endpoint works, but the agency does not document it: it can change without notice.

**Format key**

| Format | Meaning | Parser |
|---|---|---|
| CAP 1.2 | Common Alerting Protocol, OASIS standard XML for public alerts: severity, area polygon, text | one shared CAP parser |
| Atom / RSS | XML feed of entries | feed parser, then the entry content |
| JSON | REST response | `serde` types per endpoint |
| XML | agency-specific XML | `quick-xml` types per endpoint |
| SOAP | XML over the SOAP protocol (ADO.NET DataSet) | `quick-xml` |
| HTML | web page, data inside the markup or in JavaScript arrays | scraping |
| PDF | bulletin as a document | text extraction, then free-text parsing |
| free text | Portuguese prose without fields | rules, then a local model (see README, local models) |

## v0 sources

### Defesa Civil (IDAP: national public alerts)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://idapfile.mdr.gov.br/idap/api/rss/cap` | Atom; each entry holds one CAP 1.2 `<alert>` | none | undocumented ⚠ |
| `https://defesacivil.sc.gov.br/categoria/alerta/feed/` | RSS, free text (not CAP) | none | official (WordPress feed) |
| `https://www.defesacivil.pr.gov.br/alertas-vigentes` | HTML | none | official page, no feed |

- The IDAP feed carries alerts from Defesa Civil Nacional and from the state agencies of PR, SC
  and RS, with polygon, severity and the public message text.
- The feed keeps about 12 hours of alerts ⚠ and is about 10 MB (about 6.5 s per request). Poll it
  often and store the alerts.
- No published terms, rate limits or license were found.

### INMET (weather warnings)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://apiprevmet3.inmet.gov.br/avisos/rss` | RSS (Alert-AS) | none | official (linked from portal.inmet.gov.br) |
| `https://apiprevmet3.inmet.gov.br/avisos/rss/{id}` | CAP 1.2, one alert | none | official |
| `https://apiprevmet3.inmet.gov.br/avisos/ativos` | JSON with GeoJSON polygons (key `hoje`) | none | undocumented ⚠ |
| `https://apitempo.inmet.gov.br/estacoes/T` | JSON, station list | none | undocumented ⚠ |
| `https://apitempo.inmet.gov.br/token/estacao/{start}/{end}/{station}/{token}` | JSON, station data | free token by email ⚠ | undocumented ⚠ |

- The host drops the connection when the User-Agent is curl's default (`curl/...`). The honest
  `smart-alerts/<version> (+repo URL)` gets `200` on `avisos/rss`, `avisos/ativos` and
  `estacoes/T` (checked 2026-10-22).
- The RSS declares `public domain`; reproduction is allowed with citation of the source.
- `alertas2.inmet.gov.br` resets the connection from inside and outside Brazil: do not use it.

### CEMADEN

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://sws.cemaden.gov.br/PED/rest/pcds/pcds-dados-recentes` (and other `PED/rest/` routes) | JSON, observations (rain gauges, stations) | free registration on PED; JWT in the `token` header | official (docs: `https://sws.cemaden.gov.br/PED/api/`) |
| `https://sgaa.cemaden.gov.br/SGAA/rest/controle-token/tokens` | JSON, `POST` email and password, returns the JWT | PED account | official |
| `https://painelalertas.cemaden.gov.br/wsAlertas2` | JSON, municipal alerts (`alertas`, `atualizado`) | none | undocumented ⚠ |
| `http://resources.cemaden.gov.br/graficos/interativo/getJson2.php?uf=PR` | JSON, rain accumulation 1 h to 96 h | none | undocumented ⚠ |

- The official API has observations, not alerts. The municipal alerts exist only in `wsAlertas2`.
- Limit: 12 requests per minute for an external user ⚠ (from a third-party page). CEMADEN requires
  citation.

## Planned sources

### ANA (hydrological telemetry)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://www.ana.gov.br/hidrowebservice/EstacoesTelemetricas/...` | JSON (REST); river level `Cota_Adotada` in cm, flow, rain | registration by email with CPF/CNPJ; token valid 60 min | official (Swagger: `https://www.ana.gov.br/hidrowebservice/swagger-ui/index.html`) |
| `http://telemetriaws1.ana.gov.br/ServiceANA.asmx/DadosHidrometeorologicos?codEstacao=...&dataInicio=dd/mm/aaaa&dataFim=dd/mm/aaaa` | SOAP / XML, level, flow and rain every 15 min | none | legacy, no current docs ⚠ |

- Frequent token requests can block the IP: cache the token.
- A foreign applicant without a CPF or CNPJ may not be able to register ⚠.

### CPTEC/INPE (forecasts)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://servicos.cptec.inpe.br/XML/cidade/7dias/{id}/previsao.xml` | XML (ISO-8859-1), 7-day forecast | none | official (docs: `https://servicos.cptec.inpe.br/XML/`) |
| `https://servicos.cptec.inpe.br/XML/listaCidades?city={name}` | XML, city id lookup | none | official |

- Forecasts only: no warnings and no rain amount.
- The terms forbid commercial use without authorization and require the citation "CPTEC/INPE".

### Epagri/Ciram (SC)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://ciram.epagri.sc.gov.br/index.php/category/avisos-meteorologicos/feed/` | RSS, free text | none | official (WordPress feed) |
| `https://ciram.epagri.sc.gov.br/index.php/wp-json/wp/v2/posts?categories=7` | JSON (WordPress REST), free text | none | official (WordPress API) |
| `https://ciram.epagri.sc.gov.br/dadosambientaispublicos/` | file download, hourly station data | registration with CPF | official |

- Category ids: 7 = meteorological warnings, 9 = hydrological warnings, 10 = hydrological bulletins.
- The site keeps few posts: watch for changes, do not rely on the archive.

### SGB (SACE: hydrological alert systems)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://sace.sgb.gov.br/estacoes_mapa.php?bacia=brasil` | HTML, station map | none | official page |
| `https://sace.sgb.gov.br/relatorio.php?apenas_grafico=sim&bacia={basin}&...` | HTML; levels and thresholds in JavaScript arrays | none | official page |
| `https://www.sgb.gov.br/sace/boletins/{Basin}/...pdf` | PDF bulletin | none | official |

- No API. Many SACE stations are also ANA stations: the ANA telemetry can give the raw levels ⚠.

### Defesa Civil RS (replaced the SEMA-RS Sala de Situação in February 2025)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://defesacivil.rs.gov.br/avisos-e-boletins` | HTML list of PDF bulletins and warnings | none | official |
| `https://clima.rs.gov.br/climars/api/v1/ana/cotas/municipio/{ibge}` | JSON, ANA river levels with status and trend | none | undocumented ⚠ (internal API of ClimaRS) |
| `https://redehidrometeorologica.defesacivil.rs.gov.br/` | HTML | none | expired TLS certificate on 2026-10-22 |

### SIMEPAR (PR)

| Endpoint | Format | Auth | Status |
|---|---|---|---|
| `https://www.simepar.br/` | HTML behind a Cloudflare challenge | — | no API found |

- Every automatic client gets `403` (`cf-mitigated: challenge`), from inside and outside Brazil.
  The source gets no module until SIMEPAR publishes an API.

## Not verified

- Authenticated calls: CEMADEN JWT, INMET station token, ANA REST.
- Rate limits, except the CEMADEN figure ⚠.
- The license and attribution terms of IDAP, SGB, ANA and SIMEPAR.
