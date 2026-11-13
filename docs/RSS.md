<!-- reading-time: 4 min -->

# The RSS combinator

The RSS combinator reads RSS and Atom feeds and composes them with rules. A rule is an expression
in a small algebra. The result of a rule is an Atom feed, written by the Atom + CAP 1.2 channel of
`Alerts.Feed`.

The combinator never creates an alert. A rule can read the official alerts that the sources stored,
and it can filter them, but it cannot change them.

## Layout

| Path | Job |
|---|---|
| `haskell/alerts/src/Alerts/Rss/` | No I/O. The `Item` type (`Item.hs`), the RSS and Atom parser (`Parse.hs`), the rule algebra `Expr`, the evaluator `eval` and the Atom output `atom` (`Expr.hs`) |
| `haskell/alerts/src/Alerts/Store.hs` | The tables `feeds`, `feed_items` and `rules`, on SQLite |
| `haskell/alerts/app/alerts/` | `feed-add`, `rule-add`, `rss-fetch` (reads each enabled feed and stores its items) and `rss-atom <rule>` (prints the Atom feed of a rule) |

```console
ALERTS_DB=alerts.db cabal run -v0 alerts -- feed-add g1-sc https://g1.globo.com/rss/g1/sc/
ALERTS_DB=alerts.db cabal run -v0 alerts -- rule-add sul "Sul" '{"op": "feed", "feed": "g1-sc"}'
ALERTS_DB=alerts.db cabal run -v0 alerts -- rss-fetch
ALERTS_DB=alerts.db cabal run -v0 alerts -- rss-atom sul > sul.atom
```

`rss-fetch` sends `Alerts.Core.userAgent`. A feed that fails is reported, the other feeds still
run, and the command fails at the end.

## The store

| Table | Notes |
|---|---|
| `feeds` | `slug` is unique. `feed-add` with a known slug replaces the feed. There is no delete command yet. |
| `rules` | `expr` is the JSON of an `Expr`. `rule-add` evaluates it on empty feeds before it stores it: an unknown feed or rule, a bad time and a cycle are errors. |
| `feed_items` | Written only by `rss-fetch`. The same `(feed, key)` is one record. A deleted feed deletes its items (`ON DELETE CASCADE`). |

## The algebra

A rule works on item *sets*. The identity of an item is its key: the link, else the guid or Atom
id, else the title. Each result is in one canonical form: one item per key, the newest first. So
`union` and `intersect` are associative, commutative and idempotent, and `filter` distributes over
`union`. The tests in `haskell/alerts/test/Main.hs` check these laws (P5 of [`SPEC.md`](SPEC.md)).

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
- No PARA tree (users, areas, projects, tasks) and no delete command: the Rust CRUD layer had them,
  and the Haskell store keeps only the three tables above ⚠.
- No HTTP server: `rss-atom` prints the feed. Serving it is part of the hosting choice in
  `docs/HOSPEDAGEM.md`.
