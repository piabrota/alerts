---
name: generate
description: Generates the boilerplate for a new alerts app, source module, harness stage or channel, in the spirit of the Loco `generate`. Use when the user asks to add an app, a source (an agency), a stage (a filter) or a channel (a delivery target).
---

# generate

A generator writes the same files every time, so a reviewer reads only the logic. Read the `manage-apps`
skill first.

## Kinds

| Kind | Writes | Must not touch |
|---|---|---|
| `app <app>` | `apps/<app>/app.toml`, `README.md`, `evals/` | any package |
| `source <id>` | `haskell/alerts/src/Alerts/Source/<Id>.hs`, its line in `alerts.cabal`, a fixture, a `docs/SOURCES.md` row | `alerts-core` |
| `stage <name>` | one function `<name> :: ... -> Stage` and its tests | the other stages |
| `channel <id>` | `haskell/alerts/src/Alerts/<Id>.hs`, its line in `alerts.cabal` | the harness output |

There is no generator recipe yet ⚠: write the files by hand, and say so in the PR.

## Source module

```text
haskell/alerts/src/Alerts/Source/<Id>.hs   -- sourceId, the endpoint URL, parse :: ByteString -> Either Error [Alert]
haskell/alerts/test/fixtures/<id>.json     -- one recorded real response per format
```

1. Keep the module pure: `Alerts.Service` fetches the URL and calls `parse`. Add the source to
   `Alerts.Service` (`sourceIdOf` and the fetch). `Alerts.Http` sends `Alerts.Core.userAgent` on every request. Never send another
   User-Agent.
2. Parse into `Alert`. Fill only the fields that the source states.
3. An undocumented endpoint must fail loudly: return an error on an unknown shape, never an empty
   list.
4. Record the endpoint, the format, the authentication and the rate limit in `docs/SOURCES.md`.
5. A new library must be FOSS. Add it to `alerts.cabal` and to the `ghcWithPackages` list of
   `flake.nix`.
6. No partial function (`head`, `fromJust`, `error`) outside tests. The build uses `-Werror`.

## Stage

A stage is `Alert -> Maybe Alert`. Return `Nothing` to drop the alert. Do not add a class
hierarchy. Test it with one alert that passes and one alert that drops. When the stage states a
fact for every input, add its theorem to `spec/Alerts/Properties.agda`.

## Channel

A channel takes the harness output and delivers it. It formats; it does not filter. A test alert
is marked as a test (`<status>Test</status>` in CAP, a visible tag in other channels).

## After generation

1. Run `just check`.
2. Add one eval case from a real alert.
3. Mirror any README change in `README.md` and `README.pt-BR.md`.
