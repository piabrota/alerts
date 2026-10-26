---
name: map
description: Draws smart-alerts alert areas on a map with MapLibre (GL JS on the web, MapLibre Native in apps), with FOSS tiles and no API key. Use when a page, a frontend, a Discord image or a feed preview must show where an alert applies, or when CAP 1.2 areas must become GeoJSON.
---

# map

A map shows where an alert applies. The map never adds an area, a time or a severity that the
source does not state. The rules below come from `GROUND_TRUTH.md` (official sources only, FOSS
first, no paid API).

## Stack

| Part | Choice | License | Why |
|---|---|---|---|
| Web renderer | [MapLibre GL JS](https://github.com/maplibre/maplibre-gl-js) | BSD-3-Clause | The open fork of Mapbox GL JS v1. No token. |
| Native renderer | [MapLibre Native](https://github.com/maplibre/maplibre-native) | BSD-2-Clause ⚠ | Static images (for Discord) and mobile apps. |
| Base tiles | [OpenFreeMap](https://openfreemap.org) or a self-hosted [Protomaps](https://protomaps.com) PMTiles file | code MIT / BSD-3-Clause; data ODbL (OpenStreetMap) | No key and no account. PMTiles is one static file. |
| Municipal borders | IBGE malhas API (`servicodados.ibge.gov.br/api/v3/malhas/...`) | public data ⚠ (terms not checked) | The official borders of PR, SC and RS. |

- Never use a Mapbox, Google or other paid tile style. A paid tile service needs explicit human
  confirmation.
- Always show the attribution control: "© OpenStreetMap contributors" and the tile provider.
- Pin the MapLibre GL JS version (`maplibre-gl@<x.y.z>`), and load it from npm or a pinned CDN URL.

## From CAP 1.2 to GeoJSON

A CAP `<area>` can hold `<polygon>`, `<circle>` and `<geocode>`. Convert each one this way:

1. **Polygon.** CAP writes `lat,lon` pairs separated by spaces. GeoJSON writes `[lon, lat]`.
   Swap the order of every pair. A swapped polygon draws in the ocean: check one point by hand.
2. **Close the ring.** The first point and the last point of a GeoJSON ring must be equal. CAP
   requires this too, but check it.
3. **Circle.** CAP writes `lat,lon radius`, with the radius in kilometers. Approximate the circle
   with a 64-point polygon. Keep the center and the radius in the feature properties.
4. **Geocode.** A geocode names an area without a shape (for example, an IBGE municipality code ⚠).
   Get the shape from the IBGE malhas API and cache it. Never draw a guessed shape.
5. **No area.** When the alert has only text (`area` in `smart_alerts_core::Alert`), draw no
   shape. Show the UF borders and the area text instead.

Today `Alert` keeps only `area: String`. A geometry field is a core change: propose it as a new
type in `smart-alerts-core`, and keep the conversion in a `Stage` or in the channel crate.

## Style

- Color by `Severity`, with one fixed palette for all channels. Use the same color in Discord,
  the feed preview and the web map.
- Fill at 30% opacity and draw a 2 px outline, so the base map stays readable.
- Put the severity in text too (the label and the legend). Color is never the only signal.
- Fit the view to the alert bounds, with padding. Never zoom past the precision of the source.
- Write the map text in pt-BR, at the `estrito` level of `write-pt-br`.

## Minimal page

```html
<link rel="stylesheet" href="https://unpkg.com/maplibre-gl@<x.y.z>/dist/maplibre-gl.css">
<script src="https://unpkg.com/maplibre-gl@<x.y.z>/dist/maplibre-gl.js"></script>
<div id="map" style="height: 480px"></div>
<script>
  const map = new maplibregl.Map({
    container: "map",
    style: "https://tiles.openfreemap.org/styles/liberty",
    center: [-51.2, -27.5], // [lon, lat]: South Brazil
    zoom: 5,
  });
  map.on("load", () => {
    map.addSource("alerts", { type: "geojson", data: "/alerts.geojson" });
    map.addLayer({ id: "alerts-fill", type: "fill", source: "alerts",
      paint: { "fill-color": ["get", "color"], "fill-opacity": 0.3 } });
    map.addLayer({ id: "alerts-line", type: "line", source: "alerts",
      paint: { "line-color": ["get", "color"], "line-width": 2 } });
  });
</script>
```

## Check before a PR

1. One real alert from each source draws in the correct place (record it as an eval case).
2. The attribution shows.
3. No request goes to a paid service (look at the network tab).
4. The page works without the map: the alert text is complete without it.

## ⚠ claims in this skill

- The MapLibre Native license.
- The terms of the IBGE malhas API.
- IDAP geocodes are IBGE municipality codes.
