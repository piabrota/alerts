# Loco bootstrap: frontend screenshots

These screenshots show the frontend that the [Loco](https://loco.rs) bootstrap generates. Nothing
in this directory is adopted. The `manage-apps` skill (`verbs/manage-apps/`) takes the Loco conventions,
not this code.

Taken on 2026-10-26, with Chromium (Playwright) at 960 × 400 pixels.

| Screenshot | Page |
|---|---|
| [`1-home.png`](1-home.png) | `/`: the home page |
| [`2-register.png`](2-register.png) | `/register`: the sign-up form |
| [`3-register-sent.png`](3-register-sent.png) | `/register` after a sign-up: the email check message |
| [`4-login.png`](4-login.png) | `/login`: the log-in form |

## How to make them again

```console
cargo install loco --locked        # loco 1.2.0
loco new -n ialerts --db sqlite --bg async --assets clientside
cd ialerts/frontend && npm install && npm run build && cd ..
cargo loco start                   # http://localhost:5150
```

The bootstrap frontend is React with Vite and React Router. It has no style sheet. The sign-up in
`3-register-sent.png` used a test address (`teste@example.com`) on a local SQLite database: no
email was sent.

## What ialerts would keep

| Loco part | ialerts | Why |
|---|---|---|
| Generators, `doctor`, scheduler, tasks | keep, as `just` recipes (skills `generate`, `diagnose`, `schedule`) | one command per job |
| Users, JWT auth, sign-up pages | drop in v0 | the v0 channels (Discord, Atom feed) have no accounts |
| ORM (SeaORM) and migrations | drop in v0 | the service stores only the ids of delivered alerts |
| Client-side frontend | later, for a MapLibre alert map (skill `map`) | a map is not a v0 deliverable |

Loco is under the Apache-2.0 license.
