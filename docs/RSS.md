<!-- reading-time: 4 min -->

# The RSS combinator

The RSS combinator reads RSS and Atom feeds and composes them with rules. Each user has a PARA
tree: areas, projects and tasks. Each area, project or task can own feeds and rules. A rule is an
expression in a small algebra. The result of a rule is an Atom feed, written by the Atom + CAP 1.2
channel of `alerts-feed`.

The combinator never creates an alert. A rule can read the official alerts that the sources stored,
and it can filter them, but it cannot change them.

## Layout

| Path | Job |
|---|---|
| `crates/alerts-rss/` | No I/O. The `Item` type, the RSS and Atom parser, the rule algebra (`Expr`), the evaluator (`eval`) and the Atom output (`atom`) |
| `crates/alerts-crud/` | The resources `users`, `areas`, `projects`, `tasks`, `feeds`, `rules` and `feed_items`, on SQLite |
| `crates/alerts/` | `alerts rss-fetch` reads each enabled feed and stores its items. `alerts rss-atom <rule>` prints the Atom feed of a rule |

```console
ALERTS_DB=alerts.db cargo run -q -p alerts -- rss-fetch
ALERTS_DB=alerts.db cargo run -q -p alerts -- rss-atom sul > sul.atom
```

`rss-fetch` sends `alerts_core::USER_AGENT`. A feed that fails is reported, the other feeds still
run, and the command fails at the end.

## The resources

All operations go through `controllers::dispatch`, as the other resources do. The desktop app
draws a page for each resource from its spec.

| Resource | Operations | Notes |
|---|---|---|
| `users` | CRUD, `para` | No login. `para` gives the tree of the user as text, with the feeds and rules of each level. |
| `areas` | CRUD | `user_id` must exist. `slug` is unique in the user. |
| `projects` | CRUD | `area_id` must exist. `slug` is unique in the area. |
| `tasks` | CRUD | `project_id` must exist. |
| `feeds` | CRUD | `scope` is `area`, `project` or `task`, and `scope_id` must exist. `url` is HTTPS. The slug `alerts` is reserved. A feed that a rule reads cannot be deleted or renamed. |
| `rules` | CRUD, `atom` | `expr` is the JSON of an `Expr`. The controller evaluates it on empty feeds before it stores it: an unknown feed or rule, a bad time and a cycle are field errors on `expr`. `atom` evaluates the rule on the stored items. |
| `feed_items` | list, show, delete | Written only by `rss-fetch`. The same `(feed, key)` is one record. |

A delete goes down the tree. A deleted user deletes its areas, projects and tasks, and the feeds,
rules and items they own.

## The algebra

A rule works on item *sets*. The identity of an item is its key: the link, else the guid or Atom
id, else the title. Each result is in one canonical form: one item per key, the newest first. So
`union` and `intersect` are associative, commutative and idempotent, and `filter` distributes over
`union`. The tests of `crates/alerts-rss/src/expr.rs` check these laws.

| `op` | Fields | Result |
|---|---|---|
| `feed` | `feed` | The stored items of the feed with this slug |
| `alerts` | | The official alerts, each with its CAP alert |
| `rule` | `rule` | The result of another rule. A rule cannot reach itself |
| `union` | `of` | The items of any operand |
| `intersect` | `of` | The items whose key is in every operand |
| `diff` | `left`, `right` | The items of `left` whose key is not in `right` |
| `filter` | `when`, `of` | The items for which the test `when` holds |
| `map` | `with`, `of` | Each item changed. An alert item does not change |
| `dedupe` | `of` | One item per title (case and spaces ignored), the newest |
| `take` | `n`, `of` | The `n` newest items |

Tests (`is`): `contains` (`field`: `title`, `summary` or `any`; `text`), `since` (`time`, RFC 3339
or RFC 822), `from` (`feeds`), `min_severity` (`severity`), `ufs` (`ufs`), `not` (`p`), `all` and
`any` (`of`). `min_severity` and `ufs` use the stages of the harness in `alerts-core`.

Changes (`do`): `prefix_title` (`text`), `strip_html`, `truncate` (`chars`).

When two feeds have the same key, the item of the feed whose slug sorts first stays.

## Example

The flood news of two feeds and the official alerts for the South, at most 20 items:

```json
{"op": "take", "n": 20, "of": {"op": "union", "of": [
  {"op": "filter", "when": {"is": "contains", "field": "any", "text": "enchente"},
   "of": {"op": "union", "of": [{"op": "feed", "feed": "g1-sc"}, {"op": "feed", "feed": "nsc"}]}},
  {"op": "filter", "when": {"is": "ufs", "ufs": ["PR", "SC", "RS"]}, "of": {"op": "alerts"}}
]}}
```

In the Atom output, an entry from `alerts` holds its alert as CAP 1.2 in `<content>`, with the same
text as every other channel.

## Not done

- No schedule: a person or a timer runs `rss-fetch`. The `schedule` skill describes the rates.
- No HTTP server: `rss-atom` prints the feed. Serving it is part of the hosting choice in
  `READINESS.md`.
- The desktop pages use plain id fields for `user_id`, `area_id`, `project_id` and `scope_id`. A
  picker is the next step.
- Nobody ran `just desktop-check` on this change ⚠: the test machine has no WebKitGTK.
