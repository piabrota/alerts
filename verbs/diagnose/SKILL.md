---
name: diagnose
description: Checks that a smart-alerts app can run — toolchain, environment, source reachability with the honest User-Agent, rate limits and channel output — in the spirit of `cargo loco doctor`. Use before the first run of an app, after a source fails, or when the user asks why an app sends nothing.
---

# diagnose

The doctor reads and reports. It never sends a real alert and never changes a file.

## Checks, in order

| # | Check | Pass when | On failure |
|---|---|---|---|
| 1 | Toolchain | `nix develop` starts and `just check` passes | fix the code first |
| 2 | Environment | each channel of the app has its variable (`DISCORD_WEBHOOK_URL`, `FEED_PATH`) | name the missing variable; never print its value |
| 3 | Sources | each source answers with `smart_alerts_core::USER_AGENT` | stop and tell a human; never try another User-Agent |
| 4 | Shape | each answer parses with the source crate | the endpoint changed: open an issue with the response |
| 5 | Rate limits | the schedule stays under each documented limit (CEMADEN: 12 per minute) | lower the poll rate |
| 6 | Channels | `just test-message` delivers one test alert | report the HTTP status or the file error |
| 7 | Feed | the CAP output validates against the OASIS CAP 1.2 XSD | fix `smart-alerts-feed` |

## Report

Write a table: check, result (`ok`, `fail`, `skipped`), and one line of evidence (a status code, a
file path, a count). A check that did not run is `skipped`, never `ok`.
