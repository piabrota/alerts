# References: external tools and services

This page indexes external tools and services that someone proposed for smart-alerts. It is not
a list of sources: alert sources are in `docs/SOURCES.md`.

An entry here is not adopted. Each entry states what the resource is, its license and cost, and
its relevance. A new dependency must still pass `just deny`, and a paid resource needs explicit
human confirmation (`GROUND_TRUTH.md`).

Checked on 2026-10-22.

| Resource | What it is | License and cost | Relevance | Status |
|---|---|---|---|---|
| [Onyx](https://docs.onyx.app/deployment/getting_started/quickstart) | A self-hosted AI chat and search platform ([`onyx-dot-app/onyx`](https://github.com/onyx-dot-app/onyx)) | MIT, except the `ee` directories (Onyx Enterprise License) | Can index this repo's docs and sources for search by people and agents | not adopted |
| [Seabase](https://seabase.ai/infrastructure) | Subsea data centers for AI | commercial ⚠ (no pricing on the page) | Hosting only; not needed for v0 | not adopted |

## Onyx

- **License.** The root `LICENSE` puts all content outside the `ee` directories under the MIT
  (Expat) license. The `ee` directories (`backend/ee`, `web/src/app/ee`, `web/src/ee`) are under
  the Onyx Enterprise License. A smart-alerts setup uses only the MIT part ⚠ (not checked: which
  features are in `ee`).
- **Install** (from the quickstart page):

  ```bash
  uv tool install onyx-cli && onyx-cli deploy install
  ```

  The installer checks the system resources, asks for the Lite or the Standard mode and the
  version, writes its configuration to `~/.config/onyx`, and starts the containers. It installs
  Docker on Linux when Docker is missing.
- **Not on the quickstart page:** the RAM and CPU requirements, the supported models (a local
  model through Ollama ⚠), the connectors, and how indexing works.
- **Fit with the rules.** Onyx is a separate service, not a Rust dependency, so `just deny` does
  not check it. Run it locally. Do not add it to the repo as code or CI.
- **Before use:** read `docs/SURVEY_NLP.md`. Use the cheapest tool that does the job.

## Seabase

- The page is a JavaScript application. Its HTML gives only a title and a description:
  "Modular coastal and marine AI infrastructure designed for high-density GPU clusters, regional
  placement, and incremental expansion near demand."
- No pricing, API, license or self-hosting option is readable on the page ⚠.
- smart-alerts v0 is one static binary and needs no GPU. A paid host needs explicit human
  confirmation.
- Possible fit: marola (ocean work), not smart-alerts ⚠. Nobody decided this.

## ⚠ claims on this page

- Which Onyx features are in the `ee` directories.
- Onyx support for a local model through Ollama.
- Seabase pricing, API and license.
- Seabase as a fit for marola.
