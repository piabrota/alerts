---
name: schedule
description: Plans how a smart-alerts app polls its sources and runs one-off tasks (replay, eval, backfill), in the spirit of the Loco scheduler and tasks. Use when the user asks how often to poll, why alerts arrive late or twice, or wants to replay recorded alerts.
---

# schedule

## Poll rates

Take each rate from `docs/SOURCES.md`. Never poll faster than the documented limit.

| Source | Rule | Default |
|---|---|---|
| Defesa Civil (IDAP) | the feed keeps about 12 hours ⚠ and is about 10 MB | every 5 minutes |
| CEMADEN | at most 12 requests per minute for an external user ⚠ | every 30 seconds |
| INMET | no documented limit ⚠ | every 10 minutes |

## Delivery

1. Store the id of each delivered alert. Deliver an id once per channel, even after a restart.
2. Keep the order of the source. A late alert is delivered, never dropped for being late.
3. A failed poll is logged and retried at the next tick. It never sends an empty "all clear".

## Tasks

A task is a one-off job of an app. It never reaches a real channel unless the user asks.

| Task | What it does |
|---|---|
| `replay` | runs recorded alerts from `apps/<app>/evals/` through the harness and prints the output |
| `eval` | compares the replay output with the expected output; fails on a difference |
| `backfill` | reads the IDAP feed once and stores the ids, so a first start sends no old alerts |
