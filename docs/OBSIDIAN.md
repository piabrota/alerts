<!-- reading-time: 5 min -->
# The Obsidian plugin: WeatherDisaster

WeatherDisaster (plugin id `piabrota-alerts`) puts official disaster alerts into an Obsidian vault.
It reads an Atom or RSS feed of CAP 1.2 alerts and keeps one note per alert. A second brain can
then link, query and review the alerts like any other note. The plugin does not forecast the
weather: it copies what an official agency published.

## Install

```console
nix develop
just obsidian
cp -r target/obsidian/piabrota-alerts <vault>/.obsidian/plugins/
```

Then turn on WeatherDisaster in *Settings → Community plugins*. The folder holds two files:
`main.js` and `manifest.json`. The plugin is not in the community plugin list.

## What it does

1. It reads the feed at an interval (5 minutes by default). It needs no key and no paid API. Every
   request sends `alerts_core::USER_AGENT`.
2. It writes one note per CAP `<identifier>` in the folder `Alertas/`. The frontmatter holds the CAP
   fields: `cap_identifier`, `cap_sent`, `event`, `headline`, `severity`, `urgency`, `certainty`,
   `onset`, `expires`, `area_desc`, `uf`, `ibge`, `sender_name`, `web`, `state`. Dataview and Bases
   can query them.
3. A CAP `Update` rewrites the note of the alert that it references. `cap_history` keeps the
   earlier identifiers. A CAP `Cancel` sets `state: "cancelled"` and adds a *Cancelamento*
   section. The note keeps the text of the alert.
4. It drops alerts with CAP status `Exercise`, `System` or `Draft`. It drops `Test` alerts too,
   unless the setting *Alertas de teste* is on.
5. Filters: UFs, IBGE municipality codes and a lowest severity. The filters apply to a new alert.
   An `Update` or a `Cancel` always reaches a note that exists.
6. It shows a notice for each new, updated or cancelled alert. The status bar shows the number of
   active alerts: not cancelled and not expired.
7. A code block `alerts` lists the active alerts, the most severe first. Put it in a daily note:

   ````markdown
   ```alerts
   ```
   ````

The note text is the issuer's text. The plugin adds labels, never content.

## A failed read

A failed read is never "no alert". When the feed answers an error, or a document is not Atom, RSS
or CAP, the plugin shows a notice and the status bar shows `⚠ Alertas: falha na leitura`. The
`alerts` block names the feed and the error. "Nenhum alerta ativo" appears only after a read that
worked, with the feed and the time of that read. This is the rule of `docs/MAROLA.md`
("Conflict: a failed fetch").

## Feeds

The default feed is the national Civil Defense feed (IDAP), with the filter PR, SC and RS:
`https://idapfile.mdr.gov.br/idap/api/rss/cap`. The endpoint is undocumented ⚠ (see
`docs/SOURCES.md`). Nobody checked the plugin against the live IDAP feed yet ⚠.

The plugin reads three shapes:

- Atom with the CAP `<alert>` inside each entry: the feed of `alerts-feed` and of IDAP.
- Atom or RSS whose entry text is an escaped CAP document.
- Atom or RSS whose entry links to a CAP document, as the INMET RSS does. The plugin fetches
  each link once.

## How it is built

The repository is full Rust, and Obsidian loads only JavaScript. So the plugin logic is the crate
`crates/alerts-obsidian`, built to WebAssembly: the Atom, RSS and CAP reader, the filters, the note
text, the `Update` and `Cancel` rules, and the active list. `crates/alerts-obsidian/plugin/main.js`
is only the bridge to the Obsidian API: HTTP, vault files, notices, the status bar and the settings.
`just obsidian` joins the wasm-bindgen glue, the WebAssembly in base64 and that file into one
`main.js`. `cargo test -p alerts-obsidian` tests the logic without Obsidian.

The frontmatter values are JSON, which is also YAML. The Obsidian property editor can write the
values back as YAML lists ⚠, and the plugin then can no longer read the note. It then shows a
notice and does not change the note.
