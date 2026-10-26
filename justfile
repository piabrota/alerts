# Run every recipe inside `nix develop`. There is no CI: `just check` is the gate before a PR.

default:
    @just --list

# Everything a PR must pass.
check: fmt-check lint test deny reuse docs-stale pins figure-check reading-time-check

fmt:
    cargo fmt --all
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

# Build the smart-alerts Python wheel into target/wheels.
py-build:
    maturin build --release -m crates/smart-alerts-py/Cargo.toml

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
    cargo run -q -p smart-alerts-mcp

# Publish one test alert to every configured channel (DISCORD_WEBHOOK_URL, FEED_PATH).
test-message:
    cargo run -q -p smart-alerts -- test-message

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
    if [ "$(gh api repos/h0ffmann/smart-alerts --jq .private)" = "true" ]; then
        echo "The repo is private: the Wiki is deployed only after it is public (issue #1)." >&2
        exit 1
    fi
    tmp=$(mktemp -d)
    git clone -q https://github.com/h0ffmann/smart-alerts.wiki.git "$tmp"
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
    code-graph extract . --out build/graph --exclude vendor --exclude target --exclude build --exclude scripts --exclude verbs --title smart-alerts
    @echo "graph: build/graph/graph.html"

# Build the static site into site/dist/ (Gleam on JavaScript, picasso's gleam-website toolchain).
site:
    nix develop --no-update-lock-file .#site --command just --justfile site/justfile --working-directory site build

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
    cargo run -q -p smart-alerts-drawings --bin wave > docs/drawings/wave.svg
    cargo run -q -p smart-alerts-drawings --bin wave desmos > docs/drawings/wave.desmos.json

# Record Ammonite by Harry Chen (Desmos) as docs/drawings/ammonite-harry-chen.gif. Needs Chrome or
# Chromium (CHROME names the binary) and access to desmos.com.
desmos-gif *args:
    cargo run -q --release -p smart-alerts-desmos -- --out docs/drawings/ammonite-harry-chen.gif {{ args }}

# Build the Codeberg copy without AI trace into OUT (empty, or only .git). Commit it as the author.
export-codeberg out ref="HEAD":
    cargo run -q --manifest-path scripts/export-codeberg/Cargo.toml -- {{ out }} {{ ref }}

# Sync the Codeberg copy: build it on top of the Codeberg branch, commit it as the author and push.
# Flags: --remote, --branch, --ref, --author "Name <email>", --message, --sign, --dry-run.
sync-codeberg *args:
    cargo run -q --manifest-path scripts/export-codeberg/Cargo.toml --bin sync-codeberg -- {{ args }}
