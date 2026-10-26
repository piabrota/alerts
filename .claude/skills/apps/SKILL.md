---
name: apps
description: Manages smart-alerts apps — one deployment that joins sources, a harness and channels for one audience — with Loco-style conventions (new, generate, doctor, schedule, task). Use when the user wants to create, change, run, check or remove an app, or asks where an app's parts go.
---

# apps

The layout copies the idea of [Loco](https://loco.rs) ("Rust on Rails"): convention over
configuration, one command per job, and generators that write the boilerplate. It keeps only what
smart-alerts needs. There is no ORM, no mailer and no server-side HTML in v0.

## What an app is

An app is one deployment for one audience. Example: the v0 Discord server for floods in PR, SC
and RS. An app joins three parts. Each part already exists as a crate or a function.

| Part | Where it lives | Loco equivalent |
|---|---|---|
| Sources | `crates/smart-alerts-source-<id>` | models (the data comes in) |
| Harness | stages in `smart-alerts-core`, composed with `Harness::then` | controller logic |
| Channels | `crates/smart-alerts` (Discord), `crates/smart-alerts-feed` (Atom + CAP 1.2) | views |
| Schedule | `apps/<app>/app.toml`, `[schedule]` | scheduler |
| Tasks | `apps/<app>/tasks/` (replay, eval, backfill) | tasks |

## Layout (planned ⚠: nothing in `apps/` exists yet)

```text
apps/<app>/
  app.toml        # name, sources, stages, channels, schedule
  README.md       # who the audience is, in pt-BR and EN
  evals/          # recorded real alerts, the expected output
```

```toml
# apps/sul-enchentes/app.toml
name = "sul-enchentes"
hazards = ["flood"]
ufs = ["PR", "SC", "RS"]
sources = ["defesa-civil-idap", "inmet", "cemaden"]
stages = ["only_ufs", "min_severity:moderate"]
channels = ["discord", "feed"]

[schedule]
defesa-civil-idap = "every 5m"   # the feed keeps about 12 hours
cemaden = "every 30s"            # at most 12 requests per minute
inmet = "every 10m"
```

The secrets are never in `app.toml`. `DISCORD_WEBHOOK_URL` and `FEED_PATH` come from the
environment.

## Commands (planned recipes ⚠)

| Job | Recipe | Skill |
|---|---|---|
| New app | `just app-new <app>` | `app-generate` |
| New source, stage or channel | `just generate <kind> <name>` | `app-generate` |
| Check an app | `just app-doctor <app>` | `app-doctor` |
| Poll the sources | `just app-start <app>` | `app-schedule` |
| One-off job | `just app-task <app> <task>` | `app-schedule` |

Until a recipe exists, do the steps of its skill by hand, and say so in the PR.

## Rules for every app

1. Every channel of an app reads the same harness output. Never filter or rewrite per channel.
2. A new app changes no other app and does not change `smart-alerts-core`.
3. A new hazard or UF needs a source that states it. Do not widen `ufs` without one.
4. An app has eval cases before its first real message.
5. `just check` passes before a PR. There is no CI.
