<!-- reading-time: 12 min -->
# Publish the alerts as a static site

`alerts publish --out <dir> [--days <N>]` writes the stored alerts of a recent window as four plain
files. A web server shows the folder at https://alerts.piabrota.art (planned: the owner has not
deployed it yet).

The command does not read any source and does not use the network. It reads the database
(`ALERTS_DB`, default `alerts.db`) that `alerts poll` fills. A timer runs `poll` and then
`publish`.

## What `publish` writes

| File | Content |
|---|---|
| `feed.atom` | The Atom + CAP 1.2 feed of the alerts in the window. It has the same entries as the file that `poll` writes to `FEED_PATH`, but `poll` writes all stored alerts. See `docs/RSS.md` for the feed shape. |
| `alerts.json` | The alerts in the window, newest first. See below. |
| `index.html` | A page in pt-BR. It has no script and no external resource. |
| `health.json` | The time of the last run, for a watchdog. See below. |

Each file goes in through a temporary file and a rename. A reader never sees half a file. The
command creates `<dir>` when it is missing.

`FEED_ID` sets the feed id, as for `poll`. `ALERTS_SOURCES` sets the sources that the page lists
under "Fontes".

## The time window

An alert has no end time, so no alert is "active". Without a limit, the page, `alerts.json` and
`feed.atom` would grow for ever and would show old alerts as if they were current. So `publish`
shows a window.

- `--days N` sets the window. The default is `7`.
- An alert is in the window when its `sent` time is not older than `now - N days`. An alert that is
  exactly `N` days old is in the window.
- `--days 0` means no window: the files show all stored alerts.
- `N` must be a whole number. Any other value (`x`, `-1`, `1.5`) gives an error and exit code 2,
  and `publish` writes no file.
- An alert whose `sent` time is not a valid time stays in the window and goes last in the list. It
  is never dropped without notice.
- The window applies only to the files of `publish`. `poll` writes all stored alerts to
  `FEED_PATH`, and the database keeps all alerts.

## What the page shows

The page shows only what an alert holds: the headline, the hazard, the area, the severity, the
states, the send time, the source and the official link. It writes nothing that the source did not
state. It never tells the reader what to do.

- A link is a link only when it starts with `https://` or `http://`. For any other scheme the page
  shows the text and no link.
- The page escapes every text from a source.
- An alert of the `mock` source has the mark "Alerta de teste".

**Limit.** An alert has no end time, no start time, no urgency and no certainty. The page leaves
these items out. Without an end time, the page cannot tell which alerts are still in force. It
lists the alerts of the window, from the newest to the oldest, and it never calls an alert
"active". The text above the list says which alerts it holds:

| Case | Text |
|---|---|
| A window | `Alertas enviados nos últimos N dias, do mais novo ao mais antigo. A fonte não informa até quando um alerta vale.` |
| `--days 0` | `Todos os alertas guardados, do mais novo ao mais antigo. A fonte não informa até quando um alerta vale.` |
| The window has no alert, the store has some | `Sem alertas nos últimos N dias.` and the time of the last check |
| The store has no alert | `Sem dados` and the time of the last check |

## `alerts.json`

```json
{
  "generated": "2026-10-27T12:00:00+00:00",
  "window_days": 7,
  "alerts": [
    {
      "id": "1234",
      "source": "cemaden",
      "hazard": "Flood",
      "severity": "Severe",
      "ufs": ["SC"],
      "headline": "…",
      "area": "…",
      "sent": "2026-10-27T09:00:00-03:00",
      "url": null
    }
  ]
}
```

| Field | Type | Meaning |
|---|---|---|
| `generated` | string | The time of this file, in UTC, as `YYYY-MM-DDThh:mm:ss+00:00`. |
| `window_days` | number or null | The window of `--days`, in days. `null` when the run had `--days 0`. |
| `alerts` | array | The alerts in the window, newest `sent` first. An alert with a `sent` that is not a time goes last. |
| `id` | string | The id that the source gave. It is unique for one `source`. |
| `source` | string | The source id, for example `cemaden`. The `mock` source gives test alerts. |
| `hazard` | string | `Flood` or `Landslide`. |
| `severity` | string | `Minor`, `Moderate`, `Severe` or `Extreme`. |
| `ufs` | array of string | The states, in two uppercase letters. It can be empty. |
| `headline` | string | The text of the source. |
| `area` | string | The area, as the source stated it. |
| `sent` | string | The time that the source sent the alert, as the source stated it. |
| `url` | string or null | The official link of the alert, or `null`. |

The names are stable. A consumer must ignore a field that it does not know.

## `health.json`

```json
{"generated":"2026-10-27T12:00:00+00:00","window_days":7,"alerts":4,"in_window":3,"newest":"2026-10-27T11:00:00+00:00"}
```

| Field | Meaning |
|---|---|
| `generated` | The time of the run, in UTC. A watchdog must raise a warning when it is older than a few timer periods. |
| `window_days` | The window of `--days`, in days, or `null` for `--days 0`. |
| `alerts` | The number of stored alerts. The window does not change it. |
| `in_window` | The number of alerts in the window: the number that the other files show. |
| `newest` | The send time of the newest stored alert, in UTC, or `null` when the store is empty. The window does not change it. |

There is no `active` count, for the reason in "Limit" above.

## Run it on a timer

`just publish` runs `publish --out serve-alerts` for a local try. On a server, a systemd timer runs
`poll` and then `publish`. The service below has two `ExecStart` lines. systemd runs the second one
only when the first one succeeds. When `poll` fails, the files stay as they were, and `generated`
in `health.json` gets old.

`poll` needs a channel. `FEED_PATH` below is that channel. Set `DISCORD_WEBHOOK_URL` as well, if you
want the Discord channel.

**Warning.** Put tokens and webhook URLs in an `EnvironmentFile` that only root can read
(`chmod 600`). Never put them in the unit: any user can read a unit file.

`/etc/systemd/system/alerts-publish.service`:

```ini
[Unit]
Description=Read the official sources and publish the alerts site
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
DynamicUser=yes
StateDirectory=alerts
WorkingDirectory=/var/lib/alerts
# ALERTS_TOKEN_<ID> and DISCORD_WEBHOOK_URL go in this file, never in the unit.
EnvironmentFile=/etc/alerts/alerts.env
Environment=ALERTS_DB=/var/lib/alerts/alerts.db
Environment=FEED_PATH=/var/lib/alerts/poll-feed.atom
Environment=ALERTS_SOURCES=cemaden
ExecStart=/usr/local/bin/alerts poll
ExecStart=/usr/local/bin/alerts publish --out /srv/alerts-site
# The site folder belongs to the group of the web server. The service user is dynamic, so it
# joins that group to write the files.
SupplementaryGroups=nginx
ReadWritePaths=/srv/alerts-site
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
```

`/etc/systemd/system/alerts-publish.timer`:

```ini
[Unit]
Description=Publish the alerts every 10 minutes

[Timer]
OnCalendar=*:0/10
Persistent=true

[Install]
WantedBy=timers.target
```

Create the site folder once, so the web server can read it and the service can write it:

```console
install -d -m 2775 -o root -g nginx /srv/alerts-site
```

Do not put the site folder under `StateDirectory`. With `DynamicUser`, systemd keeps that folder in
`/var/lib/private`, which only root can enter, so the web server cannot read it.

⚠ Nobody has run this unit and this timer yet. Check them with `systemd-analyze verify` before you
deploy.

## What is not here

The web server part is not in this repository: nginx, the TLS certificate and the DNS records for
`alerts.piabrota.art` are in the `piabrota/nix-os` repository. The static site of
`piabrota.art` stays on Codeberg Pages (see `docs/HOSPEDAGEM.md`).
