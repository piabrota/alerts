# Run every recipe inside `nix develop`. There is no CI: `just check` is the gate before a PR.

default:
    @just --list

# Everything a PR must pass.
check: fmt-check lint test deny reuse docs-stale

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# FOSS license policy and advisories (deny.toml).
deny:
    cargo deny check

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
        python3 -m zipfile -c "target/verbs/$verb.zip" "verbs/$verb"
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
