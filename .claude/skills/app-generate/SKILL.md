---
name: app-generate
description: Generates the boilerplate for a new smart-alerts app, source crate, harness stage or channel, in the spirit of `cargo loco generate`. Use when the user asks to add an app, a source (an agency), a stage (a filter) or a channel (a delivery target).
---

# app-generate

A generator writes the same files every time, so a reviewer reads only the logic. Read the `apps`
skill first.

## Kinds

| Kind | Writes | Must not touch |
|---|---|---|
| `app <app>` | `apps/<app>/app.toml`, `README.md`, `evals/` | any crate |
| `source <id>` | `crates/smart-alerts-source-<id>/` (lib), workspace member, `docs/SOURCES.md` row | `smart-alerts-core` |
| `stage <name>` | one function `pub fn <name>(...) -> Stage` and its tests | the other stages |
| `channel <id>` | `crates/smart-alerts-<id>/` (lib), workspace member | the harness output |

## Source crate

```text
crates/smart-alerts-source-<id>/
  Cargo.toml      # version, edition, lints, license from the workspace
  src/lib.rs      # pub fn fetch(client) -> Result<Vec<Alert>, Error>
  tests/fixtures/ # one recorded real response per format
```

1. Send `smart_alerts_core::USER_AGENT` on every request. Never send another User-Agent.
2. Parse into `Alert`. Fill only the fields that the source states.
3. An undocumented endpoint must fail loudly: return an error on an unknown shape, never an empty
   list.
4. Record the endpoint, the format, the authentication and the rate limit in `docs/SOURCES.md`.
5. Use `[workspace.dependencies]`. A new dependency must pass `just deny`.
6. No `unwrap` or `expect` outside tests.

## Stage

A stage is `Fn(Alert) -> Option<Alert>`. Return `None` to drop the alert. Do not add a trait
hierarchy. Test it with one alert that passes and one alert that drops.

## Channel

A channel takes the harness output and delivers it. It formats; it does not filter. A test alert
is marked as a test (`<status>Test</status>` in CAP, a visible tag in other channels).

## After generation

1. Run `just check`.
2. Add one eval case from a real alert.
3. Mirror any README change in `README.md` and `README.pt-BR.md`.
