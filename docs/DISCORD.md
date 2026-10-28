<!-- reading-time: 11 min -->
# Discord: set up the server, test it, mock alerts

This page takes you from an empty Discord server to test alerts on your screen. Follow it in
order the first time. Every command runs on your own computer.

## A webhook, not a bot

alerts posts to Discord through a **webhook**. It needs no bot. The Discord docs say that webhooks
"do not require a bot user or authentication to use"
([Webhook resource](https://docs.discord.com/developers/resources/webhook)).

- A webhook can only post to one channel. It cannot read messages, join voice or change the
  server. A bot token can do all of that, so a webhook is the smaller risk.
- The webhook URL holds a token: `https://discord.com/api/webhooks/<id>/<token>`. Anybody with the
  URL can post in the channel. Treat the URL like a password.
- No Discord Developer Portal, no application and no bot invite. You need only the permission
  **Manage Webhooks** on the server (the owner has it).

A bot comes later only if the server needs commands, for example `/alertas SC`. Nothing in the
repository uses a bot today.

## Two servers, two variables

| Variable | Server | Used by |
|---|---|---|
| `DISCORD_WEBHOOK_URL` | The alerts server, or any server you test with | `just test-message`, `just mock-alerts`, `alerts poll` |
| `MAROLA_DISCORD_WEBHOOK_URL` | The official Marola server | `just marola-discord` (the hard rule of every PR, part of `just check`) |

Both variables come from your environment. They never enter the repository, a commit, a PR, an
issue or a log (`GROUND_TRUTH.md`, "No secrets in the repo"). The code never prints the URL: a
failed request drops the URL from its error (`crates/alerts/src/lib.rs`, `post`).

## 1. Prepare the Marola server

Skip a step when the server already has it. The menu names below are from the Discord desktop
app. Nobody checked them against the Discord help pages, which refused the request ⚠.

1. **Create the server** (only if none exists): the `+` at the bottom of the server list, then
   **Create My Own**. Name it `marola`.
2. **Create two text channels:**
   - `#alertas`: the real alerts, later, when a real source exists.
   - `#alertas-teste`: everything in this page. Every message here starts with `TESTE`.

   Use a **text** channel. A forum channel needs a `thread_name` on each message, and alerts does
   not send one, so Discord rejects the post.
3. **Make both channels read-only for people:** channel settings, **Permissions**, role
   `@everyone`, turn off **Send Messages**. A webhook still posts ⚠ (a webhook is not a member, so
   the member permissions do not apply to it; nobody checked this on the Marola server).
4. **Create the webhook:** channel settings of `#alertas-teste`, **Integrations**, **Webhooks**,
   **New Webhook**.
   - Name: `marola alertas`.
   - Avatar: a PNG of the marola logo (Discord does not take SVG ⚠).
   - Channel: `#alertas-teste`.
5. **Copy the URL:** **Copy Webhook URL**. Do not paste it in a chat, a document or a terminal
   command. Go straight to step 2 below.

Later, make a second webhook in `#alertas` for the real alerts. Never point real alerts at the
test channel, or test alerts at the real channel.

## 2. Keep the URL secret on your computer

The URL must not land in a commit or in your shell history. Pick one of the two ways.

### The `.env` file

Git ignores `.env` (and `.env.*`, except the empty template `.env.example`). `just` loads it
into every recipe (`set dotenv-load`), and the `alerts` binary and the Marola test load it too,
from the current directory or a parent. So `cargo run -p alerts -- ...` works without `just`.

```console
cp .env.example .env
chmod 600 .env                 # only your user can read it
git check-ignore .env          # prints ".env": git will never add it
```

Open `.env` in your editor and paste the URL after the `=`, with no quotes and no spaces:

```text
MAROLA_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/<id>/<token>
```

Paste it in the editor, not with `echo ... >> .env`: an `echo` lands in the shell history.

- To post the tests and the mock alerts to the Marola server too, paste the same URL in
  `DISCORD_WEBHOOK_URL`. For a test server of your own, paste that server's URL there instead.
- A variable that is already set in the shell wins over `.env`. An empty value counts as not set.
- A line that `.env` cannot parse fails without printing its text:
  `.env: invalid line near character 68 (the line is not printed: it may hold a secret)`.
- `.env` is a plain text file. A backup, a sync folder or a screen share can still copy it.

### The macOS Keychain

The login Keychain keeps the URL encrypted. Save it once. `-w` with no value makes `security`
ask for the password, so you paste the URL at a hidden prompt:

```console
security add-generic-password -a "$USER" -s marola-discord-webhook -w
```

Load it in each new terminal (or in `~/.zshrc`):

```console
export MAROLA_DISCORD_WEBHOOK_URL="$(security find-generic-password -a "$USER" -s marola-discord-webhook -w)"
```

On Linux, or for one session only, read it at a hidden prompt:

```console
read -rs MAROLA_DISCORD_WEBHOOK_URL && export MAROLA_DISCORD_WEBHOOK_URL
```

For your own test server, do the same with `DISCORD_WEBHOOK_URL` and another service name, for
example `alerts-discord-webhook`. To test against the Marola server directly:

```console
export DISCORD_WEBHOOK_URL="$MAROLA_DISCORD_WEBHOOK_URL"
```

### Check it without printing it

```console
echo "${MAROLA_DISCORD_WEBHOOK_URL:+set}"   # shell variable: prints "set"
grep -c '^MAROLA_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/' .env   # .env: prints 1
```

**If the URL leaks** (a screenshot, a commit, a paste): delete the webhook in Discord at once
(**Integrations**, **Webhooks**, the webhook, **Delete Webhook**). That kills the token. Make a
new webhook and put its URL in `.env` or in the Keychain (`security add-generic-password -U ...`).
A git history rewrite does not help: assume that a pushed URL was copied.

## 3. Get the toolchain

The workspace is Rust edition 2024, so it needs rustc 1.85 or newer.

- **With Nix:** `nix develop`, then run every command below inside it.
- **Without Nix:** install [rustup](https://rustup.rs). `rust-toolchain.toml` pins the
  toolchain, so the first `cargo` command installs it. This works only when `cargo` is rustup's
  `~/.cargo/bin/cargo`. Homebrew's `cargo` ignores the file, may be older than 1.85, and fails with
  ``feature `edition2024` is required`` or `` `resolver` setting `3` is not valid``. Put
  `~/.cargo/bin` first in `PATH` (in `~/.zshrc`), or run `brew uninstall rust`. Check with
  `rustc --version`: it prints the version in `rust-toolchain.toml`.
- `just`: `brew install just`, or `cargo install just`.

The first build takes a few minutes. The next ones take seconds.

## 4. Test 1: one test message

```console
just test-message
```

Expected output:

```text
discord: test message sent (message 1301234567890123456)
```

The number is the id of the message that Discord stored. alerts asks Discord with `wait=true`,
so the line appears only after Discord stored the message.

In `#alertas-teste` you see one embed:

| Part | Value |
|---|---|
| Title | `TESTE: esta mensagem não é um alerta real.` |
| Text | `O alerts enviou esta mensagem para testar os canais de publicação.` |
| Fields | Severidade `menor`, UF `PR, SC, RS`, Fonte `alerts` |
| Color | blue (`#1682D4`, the color of `menor`) |

## 5. Test 2: the hard rule of every PR

```console
just marola-discord
```

Expected output, in the test log:

```text
marola-discord: test alert stored as message 1301234567890123456
```

It sends the same test alert to `MAROLA_DISCORD_WEBHOOK_URL`. Every PR must pass it and write the
result in **Testado** (`GROUND_TRUTH.md`). Write the message id, never the URL.

## 6. Test 3: mock alerts

The mock source (`crates/alerts-source-mock`) makes simulated alerts. They go through the same
path as a real alert: the v0 harness, then Discord, then the store and the feed. Every mock alert
is a test:

- the headline starts with `TESTE:` (the mock adds it when it is missing);
- the source is `mock`;
- the feed publishes it with CAP status `Test`.

Use a throwaway database, so the test alerts stay out of your real `alerts.db`:

```console
export ALERTS_DB="$(mktemp -d)/alerts.db"
just mock-alerts
```

Expected output:

```text
mock: 5 read, 4 passed the harness, 4 delivered
```

The five built-in scenarios:

| Severity | UF | In Discord? | Color |
|---|---|---|---|
| menor | PR | yes | blue `#1682D4` |
| moderada | SC | yes | yellow `#F5C518` |
| severa | RS | yes | orange `#F28C28` |
| extrema | PR, SC, RS | yes | red `#D7263D` |
| severa | SP | **no**: the v0 harness keeps only PR, SC and RS | — |

So four messages reach the channel, and the SP alert shows that the harness works.

Each run is a new simulated event: the alert id holds the time of the run, so a second
`just mock-alerts` delivers four new messages. The store keeps every id it delivered, so one alert
is never sent twice.

### Your own mock alerts

Write a JSON array and name it in `MOCK_ALERTS_PATH`. An example is
`crates/alerts-source-mock/tests/fixtures/alerts.json`:

```json
[
  {
    "severity": "Moderate",
    "ufs": ["SC"],
    "headline": "nível simulado do rio acima da cota de alerta.",
    "area": "Área simulada em Santa Catarina. Este alerta não é real."
  },
  {
    "hazard": "Flood",
    "severity": "Extreme",
    "ufs": ["RS"],
    "headline": "inundação simulada em área urbana.",
    "area": "Área simulada no Rio Grande do Sul. Este alerta não é real."
  }
]
```

```console
MOCK_ALERTS_PATH=crates/alerts-source-mock/tests/fixtures/alerts.json just mock-alerts
```

```text
mock: 2 read, 2 passed the harness, 2 delivered
```

| Field | Required | Values |
|---|---|---|
| `severity` | yes | `Minor`, `Moderate`, `Severe`, `Extreme` |
| `ufs` | yes | UF codes in capitals: `"PR"`, `"SC"`, `"RS"`. Any other UF passes the parser, but the v0 harness drops it. |
| `headline` | yes | pt-BR text. `TESTE:` is added at the start. |
| `area` | yes | pt-BR text. Say that the alert is not real. |
| `hazard` | no | `Flood` (the default and, today, the only value) |

Any other field is an error. An empty array is an error: a source never answers "nothing" in
silence.

Write the text like a test: say in the text that the alert is simulated. Never copy the text of a
real alert into a mock, and never name a real street or neighbourhood as the place of a mock
flood. A reader of the channel may not read the `TESTE` prefix.

### Also write the feed

```console
FEED_PATH="$(dirname "$ALERTS_DB")/feed.atom" just mock-alerts
```

The poll then writes every stored alert to an Atom file, with CAP status `Test`:

```text
mock: 5 read, 4 passed the harness, 4 delivered
feed: 8 entries written to /var/folders/.../feed.atom
```

### See them on the map

The desktop app reads the same database. With the same `ALERTS_DB`, `just desktop-dev` shows the
mock alerts on the map, each with its `TESTE` headline. See `docs/DESKTOP.md`.

## 7. When it fails

| Message | Cause | Fix |
|---|---|---|
| `no channel: set DISCORD_WEBHOOK_URL or FEED_PATH` | No variable in this terminal, and none in `.env` | Step 2. A new terminal does not keep `export`s; `.env` stays. |
| `set MAROLA_DISCORD_WEBHOOK_URL to the webhook of the official Marola Discord` | `just marola-discord` without the variable, or with an empty value in `.env` | Step 2 |
| `.env: invalid line near character N (...)` | A line of `.env` is not `NAME=value`, for example an open quote | Fix the line in the editor; the error never prints it |
| `the webhook URL is not a Discord webhook URL` | The URL does not start with `https://discord.com/api/webhooks/` or `https://discordapp.com/api/webhooks/`. A URL from Discord Canary or PTB starts with `canary.discord.com` or `ptb.discord.com`. | Copy the URL again, or change the host to `discord.com` |
| `reqwest::Error { kind: Status(404, None) }` | The webhook was deleted, or the URL is cut short | Make a new webhook (step 1.4) |
| `Status(400, ...)` | A forum channel, or a field that Discord refuses | Use a text channel |
| `Status(429, ...)` | Too many posts in a short time (rate limit) | Wait one minute, then run again |
| `mock: the mock file has no alert` | `MOCK_ALERTS_PATH` names an empty array | Add one alert |
| ``mock: invalid mock file: unknown variant `Big`, expected one of `Minor`, `Moderate`, `Severe`, `Extreme` `` | A value outside the table above | Fix the field it names |
| `mock: 5 read, 4 passed the harness, 0 delivered` | The store already had these ids | Use a new `ALERTS_DB`, or run again |
| ``feature `edition2024` is required`` | cargo older than 1.85 | Step 3 |
| `` `resolver` setting `3` is not valid`` | cargo older than 1.85 | Step 3 |
| `warning: other users can read .../.env (mode 644)` | `.env` is readable by other users | `chmod 600 .env` (step 2) |

A failed source never looks like "no alert": `alerts poll` prints the error, ends with
`Error: "1 sources failed"` and exits with a non-zero code.

## Checklist for the first day

- [ ] The server `marola` has `#alertas` and `#alertas-teste`, both text channels, read-only for
      `@everyone`.
- [ ] The webhook `marola alertas` posts in `#alertas-teste`.
- [ ] The URL is in `.env` (`git check-ignore .env` prints `.env`) or in the Keychain, never in
      a commit or the shell history.
- [ ] The check of step 2 prints `1` (`.env`) or `set` (Keychain), never the URL.
- [ ] `just test-message` with `DISCORD_WEBHOOK_URL="$MAROLA_DISCORD_WEBHOOK_URL"`: one blue embed.
- [ ] `just marola-discord`: `test alert stored as message <id>`.
- [ ] `just mock-alerts` with a throwaway `ALERTS_DB`: four embeds, blue, yellow, orange and red;
      no SP alert.
- [ ] A mock file of your own: its alerts arrive with `TESTE:` at the start.

## ⚠ claims on this page

- The Discord menu names (the Discord help page answered `403`).
- A webhook posts in a channel where `@everyone` cannot send messages.
- Discord does not take an SVG avatar.
- The 400 and 429 rows: the codes are Discord's, but nobody saw these errors from alerts.
