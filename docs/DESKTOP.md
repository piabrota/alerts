<!-- reading-time: 7 min -->
# The desktop app

The desktop app is a window to configure alerts apps and to read the delivered alerts. It is
Tauri 2 around the Loco-style CRUD of `crates/alerts-crud`. The interface is Rust too: Leptos,
compiled to wasm32.

The app never creates an alert. An alert comes only from an official source, through
`alerts_crud::models::alerts::record`. The interface can read and delete an alert.

## Run it

```console
nix develop
just desktop-dev      # trunk serves desktop/ui on :1420, and Tauri opens it
just desktop-build    # desktop/target/release/alerts-desktop, with the interface inside
just desktop-check    # format, clippy (the interface for wasm32), licenses; part of `just check`
```

- The database is `alerts.db` in the app data directory of the system. `ALERTS_DB=<file>` names
  another file, for example a copy to test with.
- `just desktop-vendor` gets MapLibre GL JS 5.24.0 with `npm pack`, which checks the npm
  integrity hash. The files go to `desktop/ui/vendor/`, and git ignores them.
- On Linux, Tauri needs WebKitGTK 4.1. The dev shell of `flake.nix` has it.

## Layout

| Path | Loco equivalent | Job |
|---|---|---|
| `crates/alerts-crud/src/models/` | models | One file per resource: the record, its params, its checks, its spec |
| `crates/alerts-crud/src/controllers/` | controllers | One file per resource: the default CRUD, or more logic |
| `crates/alerts-crud/src/migrations/` | migrations | One file per schema change, applied in order |
| `crates/alerts-scaffold/` | `cargo loco generate scaffold` | Writes the three files of a new resource |
| `desktop/app/` | the server | Tauri 2: the window and two commands |
| `desktop/ui/` | the views | The interface: one page per resource spec, the map and the about page |

## The path of a request

```mermaid
%% figure: desktop-request
%% title: How does one click in the interface reach the database?
flowchart LR
    U["desktop/ui<br/>page from a spec"] -->|invoke crud| T["desktop/app<br/>Tauri command"]
    T -->|Request| D["controllers::dispatch"]
    D -->|resource name| R["Resource<br/>(controller)"]
    R -->|params| M["Model<br/>validate, SQL"]
    M --> S[("SQLite<br/>alerts.db")]
```

<details open>
<summary>How to read this figure</summary>

**In one sentence.** Every operation goes through one function, `controllers::dispatch`.

**How to read.** Left to right. Each arrow is labelled with what goes to the next box.

**Not shown.** The `modules` command, which returns the specs. The errors, which go back the same
path as JSON with a `kind` tag.

**Evidence.** `desktop/app/src/main.rs` and `crates/alerts-crud/src/controllers/mod.rs`.

</details>

## The resources

| Resource | Operations | Notes |
|---|---|---|
| `alerts` | list, show, delete | Written only by a source module. Same `(source, id)`: one record. |
| `apps` | CRUD, `app_toml` | Each source and each channel must exist. `app_toml` gives the `apps/<app>/app.toml` of the `manage-apps` skill. `App::harness` builds the harness. |
| `sources` | CRUD | `slug` is the id of the source crate. `poll_every` is `every <N><s, m or h>`. |
| `channels` | CRUD | `target_env` names the environment variable of the target. The secret is never in the database. |
| `users`, `areas`, `projects`, `tasks` | CRUD, `para` on users | The PARA tree of each user. See `docs/RSS.md`. |
| `feeds`, `rules`, `feed_items` | CRUD, `atom` on rules; items are read-only | The RSS combinator. See `docs/RSS.md`. |

A check that fails returns the field and a pt-BR message. The form shows the message under its
input.

## A new resource

```console
just generate scaffold rivers slug:slug name:string site:url level:int note:string?
```

1. The command writes `models/rivers.rs`, `controllers/rivers.rs` and a migration.
2. It registers the three modules at the `// scaffold:` markers.
3. The interface shows the new page. It needs no change, because it draws the page from the spec.
4. You write the pt-BR labels of `SPEC` and any other check.

The kinds are `string`, `string?` (optional), `text`, `url`, `slug`, `bool` and `int`. A migration
never changes after its merge. A change to the schema is a new migration.

## The map

The map is the home page: the app opens on it. It uses MapLibre GL JS, the OpenFreeMap
`positron` tiles (a muted base map, no key and no account) and the IBGE UF borders. The
attribution shows the tile provider, OpenStreetMap and IBGE.

The page follows the public disaster maps. INMET Alert-AS colors each state by its warning level
(yellow, orange, red), and Google Crisis Map puts the layers and the list in a side panel.

- **Color.** Each UF takes the color of the highest severity of the alerts that name it. The
  colors are `alerts_core::Severity::color`, the palette of the Discord channel.
- **Legend.** One row per severity, with its pt-BR label and its count. A click turns the
  severity on or off. The label is the signal; the color repeats it.
- **Click on a UF.** The list keeps only the alerts that name the UF. The chip `UF: RS ×` clears
  the filter.
- **Click on an alert.** The card opens, and the map outlines and fits the UFs of the alert.
- **Filters.** The source, and the test alerts (headline `TESTE`) on or off.
- **Refresh.** The page reads the database every 60 seconds, because `alerts poll` writes it
  from outside the app. The button `Atualizar` reads it at once.

`alerts_core::Alert` has only the area as text, so the map draws only the UF borders that an alert
names. A guessed shape is not allowed (the `map` skill). The view never zooms past one UF. The
list gives the area of each alert in text, and it is complete without the map. A geometry field
in the core is the next step.

`just desktop-vendor` gets the UF borders once from the IBGE malhas API (minimal quality, 27 UFs),
with the honest User-Agent, into `desktop/ui/vendor/ibge/`, which git ignores.

- **Area.** The areas of marola.dev (`marola-site` `site/areas.json`): Florianópolis (FL), Rio de
  Janeiro (RJ) and Salvador (BA). An area only moves the map, up to zoom 9. An alert still covers
  its whole UF, so the area never filters the list.

## The marola brand

alerts is the "alertas" section of marola v2: marola.dev lists it as "em breve". The interface
wears the marola look. `desktop/ui/src/marola.rs` holds the brand, read from marola on
2026-10-27:

- **The strip.** The sections of marola.dev on top of every page, as on the site, then two GitHub
  pills: `marola-dev/marola` and `piabrota/alerts`.
- **The logo.** marola logo v2 (`desktop/ui/assets/marola-logo-v2.svg`), from `marola-dev/marola`
  at `19fdf0df`, CC BY 4.0. It is the brand in the menu, the favicon and the about page.
- **The tokens.** The colors, radii and shadow of `marola-site` `DESIGN.md`: neutral chrome, one
  blue accent. The severity colors stay as they are: they are data. Inter is not bundled ⚠: the
  app uses it only when the system has it.
- **The about page.** The last item of the menu, and `sobre` in the strip. It says what the app
  is, the promises it keeps (the hard rules of `GROUND_TRUTH.md`, in plain words), the marola
  areas, one pill per repo, and the credits.

A link out of the app (the strip, a pill, the source page of an alert) opens in the system
browser through `tauri-plugin-opener`. The window never leaves the app. The capability grants
only `opener:allow-open-url` and `opener:allow-default-urls` (http, https, mailto and tel).

## Screenshots

Taken on 2026-10-27 from the release build (`just desktop-build`) under Xvfb, at 1200 × 780
pixels. The data is the example app of the `manage-apps` skill and the test alert of
`just test-message`. The network of the test machine blocks the tile server, so the map shows no
base map. Nobody checked the base map on an open network ⚠.

| Screenshot | Page |
|---|---|
| [`alerts.png`](screenshots/desktop/alerts.png) | Alerts: read-only, with the test alert |
| [`apps.png`](screenshots/desktop/apps.png) | Apps: the table |
| [`edit.png`](screenshots/desktop/edit.png) | Apps: the edit form, with choices and references |
| [`invalid.png`](screenshots/desktop/invalid.png) | Apps: the field errors from the controller |
| [`app-toml.png`](screenshots/desktop/app-toml.png) | Apps: the `app_toml` action |
| [`sources.png`](screenshots/desktop/sources.png) | Sources: the table |
| [`map.png`](screenshots/desktop/map.png) | Map, the home page: the UFs colored by severity, the legend and the list (see below) |

`map.png` and the images in `screenshots/mapa-inicio/` are newer. They were taken on 2026-10-27
in Google Chrome (Playwright), at 1200 × 780 pixels, from the wasm build of the interface with a
stub of the Tauri commands. The alerts are simulated: each headline starts with `TESTE`. Nobody
took them in the Tauri window ⚠. The images in `screenshots/marola-v2/` (the marola strip, the about page,
the area picker) were taken the same way.
