# PITCH: Apply to YC Winter 2027

The plan to apply to Y Combinator, batch Winter 2027 (W27). The goal: a 3-minute demo video
that shows the power of smart-alerts to the YC partners.

Written 2026-10-22. Delete this file, or move it out of the repo, after the application.

## Dates

| Date | What |
|---|---|
| 2026-10-22 | This plan. |
| 2026-10-23 – 2026-10-31 | Build and record (days 1–9). |
| 2026-11-01 | Content day: write the answers, record the videos (day 10). |
| **2026-11-02, 20:00 PT** | **On-time deadline** ⚠ (secondary sources, not YC's page). |
| 2026-12-11 | Decision for on-time applications ⚠. |

- [ ] Confirm the deadline on `https://www.ycombinator.com/apply`.
- [ ] Convert 20:00 PT to Brasília time. US daylight time ends on 2026-11-01 ⚠, so 20:00 PST is
      01:00 BRT on 2026-11-03. Do not plan for that margin: submit on 2026-11-02.
- [ ] Submit one day early if possible. A late application is still read, but YC does not
      promise a date ⚠.

## Two videos, not one

The YC application has two different video slots ⚠ (check on the form):

| Video | Length | Content |
|---|---|---|
| Founder video | 1 minute | Only the founders talking. No demo, no slides. Speak from bullet points, not from a script. |
| Product demo | **3 minutes** (our target; check the form limit ⚠) | The product working. This is the roadmap video. |

- [ ] Confirm both slots and their limits on the form.
- [ ] Do not put the demo in the founder video.

## Hard rules for the demo

The demo follows `AGENTS.md`. A demo that breaks a rule costs more than it gains.

- [ ] Every alert in the video comes from an official source: a real recorded alert, with its
      source link visible.
- [ ] A test alert is labelled as a test: `<status>Test</status>` in CAP, and "teste" in the
      Discord text. Never stage a fake flood alert.
- [ ] Every HTTP request sends the honest `USER_AGENT`.
- [ ] No secret on screen: hide `DISCORD_WEBHOOK_URL`, tokens and the CEMADEN JWT.
- [ ] Never claim a number that nobody checked (users, alerts per day, lead time). A claim in the
      video follows the same `⚠` rule as the docs.

## Critical path: a real source

On 2026-10-22 there is no `crates/smart-alerts-source-*` crate. The engine has the core, the
Discord service, the Atom + CAP feed, the Python package and the MCP server, but nothing reads a
real alert yet. The demo needs at least one source.

- [ ] **Day 1–3: `smart-alerts-source-idap`.** IDAP is CAP 1.2 inside Atom and covers Defesa
      Civil of PR, SC and RS (`docs/SOURCES.md`). CAP is already in the feed crate's domain.
- [ ] Poll IDAP often and store what it reads: the feed keeps about 12 hours ⚠.
- [ ] Save 5–10 real South alerts as test fixtures and eval cases (`docs/SURVEY_NLP.md`).
- [ ] Fail loudly on a fetch error: never show "no alert" after a failed fetch (`docs/MAROLA.md`).
- [ ] **Day 4–5 (optional): `smart-alerts-source-inmet`.** RSS index plus CAP per alert. Do
      this only if IDAP is done.
- [ ] Skip CEMADEN for the demo: it needs a PED account and a JWT.
- [ ] `just check` is green before each PR.

## Demo flow (3 minutes)

One continuous story: a real alert goes from an agency to a person.

| Time | Scene | Shows |
|---|---|---|
| 0:00–0:20 | The problem | Brazilian alert data is scattered across agencies and formats. A person gets the alert late, or not at all. |
| 0:20–1:00 | Ingest | `smart-alerts` reads a real Defesa Civil alert (IDAP, CAP 1.2). Show the source document and its link. |
| 1:00–1:40 | Deliver | The same alert in Discord, in pt-BR, and in the Atom + CAP feed. Same text in both channels (parity). |
| 1:40–2:20 | Build on it | `import smart_alerts` in Python; an agent asks the MCP server "active alerts in RS?". |
| 2:20–2:45 | Why it holds | Official sources only, honest identification, no model invents an alert, one Rust binary. |
| 2:45–3:00 | Next | More sources and hazards, more states, more channels (the AstrBot plugin). |

- [ ] Write the voice-over as bullet points per scene.
- [ ] Record each scene as a separate clip, so one failed take costs one scene.
- [ ] Use a recorded alert for the scenes. A live alert can be absent on recording day.
- [ ] Record a backup run of every terminal scene on days 7–9.
- [ ] Keep the text large enough to read at 1080p.

## Application answers

Draft each answer before day 10. Day 10 only polishes and records.

- [ ] Company name and a one-line description ⚠ (check the character limit).
- [ ] What you make: a generic alert engine that collects official alerts, filters them and
      delivers them.
- [ ] Why now, and why Brazil first.
- [ ] Progress: what runs today, with links to the commits and PRs. Only facts.
- [ ] Users and traction: state the real number, even if it is zero.
- [ ] Competitors and alternatives: the official agency apps and channels, SMS alerts, and other
      aggregators. Say why smart-alerts complements official channels and does not replace them.
- [ ] How you make money. Keep it consistent with "FOSS first" and the MPL-2.0 license.
- [ ] Founders: background, time commitment, and who writes the code.
- [ ] Equity, legal entity and any investment ⚠ (the form asks; prepare the facts).
- [ ] Links: the repository (is it public? issue #1), the Discord server, the demo video.

## Day plan

| Day | Date | Work | Done when |
|---|---|---|---|
| 1 | 10-23 | IDAP source: parse CAP from Atom | a fixture parses into `Alert` |
| 2 | 10-24 | IDAP source: poll and store | it runs for a day and stores alerts |
| 3 | 10-25 | IDAP into Discord and the feed | one real alert reaches both channels |
| 4 | 10-26 | INMET source, or harden IDAP | `just check` green |
| 5 | 10-27 | Python and MCP show the stored alerts | the scene 1:40–2:20 works |
| 6 | 10-28 | Draft all application answers | every answer has a draft |
| 7 | 10-29 | Record backup clips of every scene | clips saved |
| 8 | 10-30 | Repo ready to show: README, public or not | a stranger can follow the README |
| 9 | 10-31 | Buffer for what slipped | – |
| 10 | 11-01 | Content and both videos | videos uploaded, answers final |
| – | 11-02 | Submit | confirmation e-mail received |

## Risks

- **No alert on recording day.** A flood alert in the South is not certain on a given day. Use a
  stored real alert from days 2–9.
- **One day for content and video is tight.** A failed recording on 11-01 leaves no time. The
  backup clips from day 7 are the fallback.
- **The repo is private** (issue #1). Decide before day 8 if YC gets a public link.
- **IDAP is undocumented** ⚠. It can change during the 10 days. The fixtures keep the demo
  working even if the live feed breaks.

## ⚠ claims on this page

- The deadline (2026-11-02, 20:00 PT) and the decision date (2026-12-11) come from secondary
  sources, not from YC's page.
- The two video slots and their length limits.
- The end of US daylight time on 2026-11-01.
- The character limits and the form fields.
- IDAP keeps about 12 hours of alerts.
