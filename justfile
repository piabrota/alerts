# Run every recipe inside `nix develop`. There is no CI: `just check` is the gate before a PR.

# Load .env (git ignores it) into every recipe: the Discord webhooks live there. See .env.example.
set dotenv-load

default:
    @just --list

# Everything a PR must pass.
check: fmt-check lint test deny reuse docs-stale pins figure-check reading-time-check desktop-check obsidian-check marola-discord

fmt:
    cargo fmt --all
    cargo fmt --manifest-path desktop/Cargo.toml --all
    cargo fmt --manifest-path scripts/export-codeberg/Cargo.toml

fmt-check:
    cargo fmt --all --check
    cargo fmt --manifest-path scripts/export-codeberg/Cargo.toml --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --manifest-path scripts/export-codeberg/Cargo.toml --all-targets -- -D warnings

test:
    cargo test --workspace
    cargo test --manifest-path scripts/export-codeberg/Cargo.toml

# FOSS license policy and advisories (deny.toml).
deny:
    cargo deny check
    cargo deny --manifest-path scripts/export-codeberg/Cargo.toml check

# Copyright and license of every file (REUSE.toml) and the citation file (CITATION.cff).
reuse:
    uvx reuse@6.2.0 lint
    cffconvert --validate

# Build the alerts Python wheel into target/wheels.
py-build:
    maturin build --release -m crates/alerts-py/Cargo.toml

# Zip each skill in verbs/ into target/verbs/<verb>.zip, to upload as a project skill.
pack-verbs:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf target/verbs && mkdir -p target/verbs
    for dir in verbs/*/; do
        verb=$(basename "$dir")
        (cd verbs && zip -qr "../target/verbs/$verb.zip" "$verb")
    done
    ls target/verbs

# Run the MCP server on stdio.
mcp:
    cargo run -q -p alerts-mcp

# Publish one test alert to every configured channel (DISCORD_WEBHOOK_URL, FEED_PATH).
test-message:
    cargo run -q -p alerts -- test-message

# MOCK_ALERTS_PATH names a JSON file of mock alerts in place of the built-in scenarios;
# crates/alerts-source-mock/tests/fixtures/alerts.json is an example. Each alert is delivered once per id.
# Deliver the mock alerts (TESTE, CAP status Test) to DISCORD_WEBHOOK_URL and/or FEED_PATH.
mock-alerts:
    cargo run -q -p alerts -- poll mock

# Hard rule: deliver one test alert to the official Marola Discord. Part of `just check`. The webhook
# comes from MAROLA_DISCORD_WEBHOOK_URL and never enters the repository.
marola-discord:
    cargo test -q -p alerts --test marola_discord -- --ignored --nocapture

# Build the Obsidian plugin WeatherDisaster (id piabrota-alerts) into target/obsidian/piabrota-alerts:
# main.js is the wasm-bindgen glue, the WebAssembly of alerts-obsidian in base64, and plugin/main.js.
# Copy that folder to <vault>/.obsidian/plugins/ to install the plugin.
obsidian:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q --release -p alerts-obsidian --target wasm32-unknown-unknown
    out=target/obsidian/piabrota-alerts; glue=target/obsidian/glue
    rm -rf "$out" "$glue" && mkdir -p "$out"
    wasm-bindgen --target no-modules --no-typescript --out-dir "$glue" target/wasm32-unknown-unknown/release/alerts_obsidian.wasm
    { cat "$glue/alerts_obsidian.js"
      printf 'const WASM_BASE64 = "%s";\n' "$(base64 < "$glue/alerts_obsidian_bg.wasm" | tr -d '\n')"
      cat crates/alerts-obsidian/plugin/main.js; } > "$out/main.js"
    cp crates/alerts-obsidian/plugin/manifest.json "$out/"
    echo "obsidian: $out"

# The plugin builds, its main.js parses, and the wasm32 lints pass. Part of `just check`.
obsidian-check: obsidian
    cargo clippy --release -p alerts-obsidian --target wasm32-unknown-unknown -- -D warnings
    node --check target/obsidian/piabrota-alerts/main.js

# List zh/ja wiki pages whose English source changed after translation.
docs-stale:
    #!/usr/bin/env bash
    set -euo pipefail
    stale=0
    for f in wiki/zh/*.md wiki/ja/*.md; do
        meta=$(grep -m1 '^<!-- source: ' "$f") || { echo "no source line: $f"; stale=1; continue; }
        src=$(sed -E 's/^<!-- source: ([^ ]+) sha: ([0-9a-f]+).*/\1/' <<<"$meta")
        sha=$(sed -E 's/^<!-- source: ([^ ]+) sha: ([0-9a-f]+).*/\2/' <<<"$meta")
        [ "$(git hash-object "$src")" = "$sha" ] || { echo "stale: $f (source $src changed)"; stale=1; }
    done
    exit $stale

# Copy wiki/ to the GitHub Wiki. Only after the repo is public: see issue #1.
wiki-sync:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(gh api repos/piabrota/alerts --jq .private)" = "true" ]; then
        echo "The repo is private: the Wiki is deployed only after it is public (issue #1)." >&2
        exit 1
    fi
    tmp=$(mktemp -d)
    git clone -q https://github.com/piabrota/alerts.wiki.git "$tmp"
    rsync -a --delete --exclude .git wiki/ "$tmp/"
    git -C "$tmp" add -A
    git -C "$tmp" commit -qm "sync wiki/ from $(git rev-parse --short HEAD)" || echo "wiki: nothing to sync"
    git -C "$tmp" push -q

# The pinned Archify (vendor/archify, sparse to archify/): once per clone.
figure-init:
    git submodule update --init --depth 1 vendor/archify
    git -C vendor/archify sparse-checkout set --no-cone /archify/
    @echo "figure-init: $(git -C vendor/archify rev-parse --short HEAD)"

# Draw every docs/archify/<slug>.json to docs/img/<slug>.png, and its English twin docs/archify/en/<slug>.json
# to docs/img/en/<slug>.png: Archify validates and checks it, picasso's look (docs/archify/picasso.css) goes
# over it, headless Chrome takes the picture. Needs node and Chrome (CHROME names another). From ptbr-writer.
figure: figure-init
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p build/figure docs/img/en
    for src in docs/archify/*.json docs/archify/en/*.json; do
        slug="${src#docs/archify/}"; slug="${slug%.json}"; out="build/figure/$slug"
        rm -rf "$out"; mkdir -p "$out"
        ARCHIFY_UPDATE_CHECK_DISABLED=1 node vendor/archify/archify/bin/archify.mjs finalize "$(jq -r .diagram_type "$src")" "$src" "$out/page.html" --out-dir "$out/receipts" --quality showcase
        read -r w h < <(grep -o '<svg viewBox="[^"]*"' "$out/page.html" | head -1 | sed -E 's/.*0 0 ([0-9.]+) ([0-9.]+)"/\1 \2/')
        crop="$(awk -F'\t' -v s="$(basename "$slug")" '$1==s {print $2}' docs/archify/crop.tsv)"; h="${crop:-$h}"
        FONTS='<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Fjalla+One&family=Inter:wght@400;600&display=block">' CSS="$(cat docs/archify/picasso.css)" W="$w" H="$h" \
            perl -0pe 's|</head>|$ENV{FONTS}<style>$ENV{CSS}</style></head>|; s|<svg viewBox="0 0 [0-9.]+ [0-9.]+"|<svg viewBox="0 0 $ENV{W} $ENV{H}"|' "$out/page.html" > "$out/frame.html"
        "${CHROME:-google-chrome}" --headless --disable-gpu --hide-scrollbars --force-dark-mode --force-device-scale-factor=2 --user-data-dir="$PWD/$out/profile" --window-size="$w,$h" --virtual-time-budget=3000 --screenshot="$PWD/docs/img/$slug.png" "file://$PWD/$out/frame.html" 2>/dev/null
        echo "figure: docs/img/$slug.png"
    done
    just _figure-lock > docs/archify/figures.lock

# The inputs of each figure (its source, the look, the crop, the Archify pin), one line per slug.
_figure-lock:
    #!/usr/bin/env bash
    set -euo pipefail
    pin="$(git ls-files -s vendor/archify | cut -d' ' -f2)"
    for src in docs/archify/*.json docs/archify/en/*.json; do
        slug="${src#docs/archify/}"
        printf '%s\t%s\n' "${slug%.json}" "$(cat "$src" docs/archify/picasso.css docs/archify/crop.tsv | sha256sum | cut -c1-16)-${pin:0:7}"
    done

# Offline: each figure has its PNG and its English twin (the same drawing, only the words changed), both
# match the last `just figure`, README.pt-BR.md embeds the pt-BR PNG and README.md the English one.
figure-check:
    #!/usr/bin/env bash
    set -euo pipefail
    shape='{d: .diagram_type, l: [.lanes[].id], m: .mainPath, n: [.nodes[] | del(.label, .sublabel, .tag)], e: [.edges[] | del(.label)]}'
    for src in docs/archify/*.json; do
        s="$(basename "$src" .json)"; en="docs/archify/en/$s.json"
        test -s "docs/img/$s.png" || { echo "figure-check: no PNG for $src; run just figure" >&2; exit 1; }
        test -s "$en" && test -s "docs/img/en/$s.png" || { echo "figure-check: no English twin $en or its PNG" >&2; exit 1; }
        test "$(jq -r .meta.locale "$en")" = en || { echo "figure-check: $en is not meta.locale en" >&2; exit 1; }
        cmp -s <(jq -S "$shape" "$src") <(jq -S "$shape" "$en") || { echo "figure-check: $en is not the same drawing as $src" >&2; exit 1; }
        ! grep -qF "docs/img/$s.png" README.md || { echo "figure-check: README.md embeds the pt-BR PNG; use docs/img/en/$s.png" >&2; exit 1; }
        ! grep -qF "docs/img/en/$s.png" README.pt-BR.md || { echo "figure-check: README.pt-BR.md embeds the English PNG; use docs/img/$s.png" >&2; exit 1; }
    done
    for en in docs/archify/en/*.json; do test -s "docs/archify/$(basename "$en")" || { echo "figure-check: $en has no pt-BR source" >&2; exit 1; }; done
    just _figure-lock | cmp -s - docs/archify/figures.lock || { echo "figure-check: a source, the look or the Archify pin changed since the PNG; run just figure" >&2; exit 1; }
    echo "figure-check: ok"

# The files for humans: READMEs, docs/ and wiki/ pages (not the agent files, not the licenses).
_human-files:
    @git ls-files 'README*.md' 'docs/**.md' 'docs/*.md' 'wiki/**.md' | grep -v '/_Sidebar.md$' | sort -u

# Write `<!-- reading-time: N min -->` in every file for humans, with ptbr-writer's calculator (`ptbr
# reading-time`). A zh or ja page takes the N of its English source, after its `source` line.
reading-time:
    #!/usr/bin/env bash
    set -euo pipefail
    n_of() { local lang=en; case "$1" in *pt-BR*|docs/REFERENCES.md) lang=pt-BR;; esac
        grep -v "^<!-- reading-time:" "$1" | ptbr reading-time --lang "$lang" | sed -E 's/^[^~]*~([0-9]+) min.*/\1/; s/^[^:]+: <? ?([0-9]+).*/\1/'; }
    for f in $(just _human-files); do
        case "$f" in wiki/zh/*|wiki/ja/*)
            src=$(sed -nE '1,3s/^<!-- source: ([^ ]+) .*/\1/p' "$f"); n=$(n_of "$src")
            grep -v '^<!-- reading-time:' "$f" | awk -v n="$n" 'NR==1{print; print "<!-- reading-time: " n " min -->"; next} {print}' > "$f.tmp";;
        *)  n=$(n_of "$f")
            { echo "<!-- reading-time: $n min -->"; grep -v '^<!-- reading-time:' "$f"; } > "$f.tmp";;
        esac
        mv "$f.tmp" "$f"; echo "$f: $n min"
    done

# Every file for humans states its reading time (`just reading-time` writes it).
reading-time-check:
    #!/usr/bin/env bash
    set -euo pipefail
    bad=0
    for f in $(just _human-files); do grep -qm1 '^<!-- reading-time: [0-9]\+ min -->$' "$f" || { echo "reading-time-check: $f has no reading time; run just reading-time" >&2; bad=1; }; done
    exit $bad

# The code graph of this repository (picasso's code-graph, graphify's schema): build/graph/graph.html.
graph:
    code-graph extract . --out build/graph --exclude vendor --exclude target --exclude build --exclude scripts --exclude verbs --title alerts
    @echo "graph: build/graph/graph.html"

# Build the static site into site/serve/ (Gleam on JavaScript, picasso's gleam-website toolchain), with
# the Obsidian plugin in alerts/obsidian/ and a feed with one test alert in alerts/feed.atom.
site: obsidian
    nix develop --no-update-lock-file .#site --command just --justfile site/justfile --working-directory site build
    mkdir -p site/serve/alerts/obsidian
    cp target/obsidian/piabrota-alerts/main.js target/obsidian/piabrota-alerts/manifest.json site/serve/alerts/obsidian/
    # The site gets the feed only: an empty DISCORD_WEBHOOK_URL wins over .env and counts as not set.
    env DISCORD_WEBHOOK_URL= FEED_PATH=site/serve/alerts/feed.atom FEED_ID=https://piabrota.art/alerts/feed.atom cargo run -q -p alerts -- test-message

# Publish the site to Codeberg Pages: build it, check it has no AI trace, and push it as one
# commit to the `pages` branch of REMOTE (default: the public repository piabrota/pages). The
# branch holds only the built site, never this repository's history. See docs/HOSPEDAGEM.md.
deploy-site remote="https://codeberg.org/piabrota/pages.git": site
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -q --manifest-path scripts/export-codeberg/Cargo.toml -- --check site/serve
    tmp=$(mktemp -d)
    cp -r site/serve/. "$tmp/"
    git -C "$tmp" init -q -b pages
    git -C "$tmp" add -A
    git -C "$tmp" commit -qm "site $(git rev-parse --short HEAD)"
    git -C "$tmp" push -f "{{ remote }}" pages
    rm -rf "$tmp"
    echo "deploy-site: pushed to {{ remote }} (branch pages)"

# Build the site and serve it on http://localhost:8000/alerts/.
site-serve: site
    miniserve site/serve --index index.html --port 8000

# The three picasso pins in flake.lock (gleam-website, code-graph, ptbr-tools) are one revision.
pins:
    #!/usr/bin/env bash
    set -euo pipefail
    revs=$(jq -r '.nodes | ["gleam-website","code-graph","ptbr-tools"] as $k | [$k[] as $n | .[$n].locked.rev] | unique | .[]' flake.lock)
    [ "$(wc -l <<<"$revs")" = 1 ] || { echo "pins: picasso revisions differ: $revs" >&2; exit 1; }
    echo "pins: picasso ${revs:0:7}"

# Write the README drawing (docs/drawings/wave.svg) and its Desmos state (wave.desmos.json).
wave:
    cargo run -q -p alerts-drawings --bin wave > docs/drawings/wave.svg
    cargo run -q -p alerts-drawings --bin wave desmos > docs/drawings/wave.desmos.json

# Record Ammonite by Harry Chen (Desmos) as docs/drawings/ammonite-harry-chen.gif. Needs Chrome or
# Chromium (CHROME names the binary) and access to desmos.com.
desmos-gif *args:
    cargo run -q --release -p alerts-desmos -- --out docs/drawings/ammonite-harry-chen.gif {{ args }}

# New resource, source, stage or channel. Only `scaffold` exists today: `just generate scaffold rivers
# slug:slug name:string level:int` writes the model, the controller and the migration of alerts-crud.
generate kind *args:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ kind }}" in
        scaffold) cargo run -q -p alerts-scaffold --bin scaffold -- {{ args }}; cargo fmt -p alerts-crud;;
        *) echo "generate: only scaffold exists; see the generate skill for the other kinds" >&2; exit 2;;
    esac

# The desktop map's files, in desktop/ui/vendor (ignored by git): MapLibre GL JS, pinned and checked by
# npm, and the IBGE UF borders (malhas API, minimal quality), fetched once with the honest User-Agent.
desktop-vendor:
    #!/usr/bin/env bash
    set -euo pipefail
    dir=desktop/ui/vendor/maplibre; tmp=$(mktemp -d)
    (cd "$tmp" && npm pack --silent maplibre-gl@5.24.0 >/dev/null && tar -xzf maplibre-gl-5.24.0.tgz)
    mkdir -p "$dir" && cp "$tmp"/package/dist/maplibre-gl.{js,css} "$tmp"/package/LICENSE.txt "$dir"/
    rm -rf "$tmp"; echo "desktop-vendor: $dir (maplibre-gl 5.24.0)"
    ibge=desktop/ui/vendor/ibge; mkdir -p "$ibge"
    if [ ! -s "$ibge/ufs.geojson" ]; then
        version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "alerts-core") | .version')
        curl -fsS --retry 2 -A "piabrota-alerts/$version (+https://github.com/piabrota/alerts)" -o "$ibge/ufs.geojson.tmp" \
            'https://servicodados.ibge.gov.br/api/v3/malhas/paises/BR?formato=application/vnd.geo%2Bjson&qualidade=minima&intrarregiao=UF'
        jq -e '.features | length == 27' "$ibge/ufs.geojson.tmp" >/dev/null
        mv "$ibge/ufs.geojson.tmp" "$ibge/ufs.geojson"
    fi
    echo "desktop-vendor: $ibge/ufs.geojson (IBGE, 27 UFs)"
    trilhas=desktop/ui/vendor/trilhas; mkdir -p "$trilhas"
    cargo run -q -p alerts-trilhas -- geojson < data/raw/trilhas/osm.tsv > "$trilhas/trilhas.geojson"
    echo "desktop-vendor: $trilhas/trilhas.geojson (OpenStreetMap, $(($(wc -l < data/raw/trilhas/osm.tsv) - 1)) trilhas)"

# Fetch the OpenStreetMap trails of ssa, rio and fln from Overpass into data/raw/trilhas/osm.tsv.
trilhas-osm:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "alerts-core") | .version')
    out=data/raw/trilhas/osm.tsv; tmp=$(mktemp)
    printf 'cidade\tpraia\tosm\tkm\tnome\tlinha\n' > "$tmp"
    for cidade in ssa rio fln; do
        cargo run -q -p alerts-trilhas -- osm-query "$cidade" \
            | curl -fsS --retry 2 -A "piabrota-alerts/$version (+https://github.com/piabrota/alerts)" \
                --data-urlencode data@- https://overpass-api.de/api/interpreter \
            | cargo run -q -p alerts-trilhas -- osm "$cidade" >> "$tmp"
    done
    mv "$tmp" "$out"
    echo "trilhas-osm: $out ($(($(wc -l < "$out") - 1)) trilhas, © OpenStreetMap contributors, ODbL)"

# Run the desktop app with live reload: trunk serves the interface on :1420, Tauri opens it.
desktop-dev: desktop-vendor
    #!/usr/bin/env bash
    set -euo pipefail
    (cd desktop/ui && trunk serve) & trap "kill $!" EXIT
    cargo run --manifest-path desktop/Cargo.toml -p alerts-desktop

# Build the desktop binary with the interface inside: desktop/target/release/alerts-desktop.
desktop-build: desktop-vendor
    cd desktop/ui && trunk build --release
    cargo build --release --manifest-path desktop/Cargo.toml -p alerts-desktop --features custom-protocol

# The gate of desktop/: format, lints (the interface for wasm32), licenses. Part of `just check`.
desktop-check:
    cargo fmt --manifest-path desktop/Cargo.toml --all --check
    cargo clippy --manifest-path desktop/Cargo.toml -p alerts-ui --target wasm32-unknown-unknown -- -D warnings
    cargo clippy --manifest-path desktop/Cargo.toml -p alerts-desktop --all-targets -- -D warnings
    cargo deny --manifest-path desktop/Cargo.toml check

# Build the Codeberg copy without AI trace into OUT (empty, or only .git). Commit it as the author.
export-codeberg out ref="HEAD":
    cargo run -q --manifest-path scripts/export-codeberg/Cargo.toml -- {{ out }} {{ ref }}
