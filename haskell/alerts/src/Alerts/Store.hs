{-# LANGUAGE OverloadedStrings #-}

-- | The store of the service (SQLite, @ALERTS_DB@): the delivered alerts, the RSS feeds that the
-- rules read, their items, and the rules.
module Alerts.Store
  ( Store,
    open,
    openMemory,
    close,
    findAlert,
    recordAlert,
    listAlerts,
    FeedRow (..),
    addFeed,
    listFeeds,
    recordItems,
    itemsOf,
    RuleRow (..),
    addRule,
    listRules,
    findRule,
  )
where

import Alerts.Core
import Alerts.Rss.Expr (Expr)
import Alerts.Rss.Item
import Data.Aeson (eitherDecodeStrict, encode)
import Data.ByteString.Lazy qualified as BL
import Data.List (intercalate)
import Data.Maybe (mapMaybe)
import Data.Text qualified as T
import Data.Text.Encoding qualified as TE
import Database.SQLite.Simple hiding (close, open)
import Database.SQLite.Simple qualified as S

newtype Store = Store Connection

schema :: [Query]
schema =
  [ "PRAGMA foreign_keys = ON",
    "CREATE TABLE IF NOT EXISTS alerts (\
    \ id INTEGER PRIMARY KEY, alert_id TEXT NOT NULL, source TEXT NOT NULL, hazard TEXT NOT NULL,\
    \ severity TEXT NOT NULL, ufs TEXT NOT NULL, headline TEXT NOT NULL, area TEXT NOT NULL,\
    \ sent TEXT NOT NULL, url TEXT,\
    \ created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),\
    \ updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),\
    \ UNIQUE (source, alert_id))",
    "CREATE TABLE IF NOT EXISTS feeds (\
    \ id INTEGER PRIMARY KEY, slug TEXT NOT NULL UNIQUE, title TEXT NOT NULL, url TEXT NOT NULL,\
    \ enabled INTEGER NOT NULL DEFAULT 1)",
    "CREATE TABLE IF NOT EXISTS feed_items (\
    \ id INTEGER PRIMARY KEY,\
    \ feed TEXT NOT NULL REFERENCES feeds (slug) ON DELETE CASCADE ON UPDATE CASCADE,\
    \ key TEXT NOT NULL, title TEXT NOT NULL, link TEXT, summary TEXT NOT NULL, updated TEXT,\
    \ UNIQUE (feed, key))",
    "CREATE TABLE IF NOT EXISTS rules (\
    \ id INTEGER PRIMARY KEY, slug TEXT NOT NULL UNIQUE, title TEXT NOT NULL, expr TEXT NOT NULL)"
  ]

open :: FilePath -> IO Store
open path = do
  c <- S.open path
  mapM_ (execute_ c) schema
  pure (Store c)

openMemory :: IO Store
openMemory = Alerts.Store.open ":memory:"

close :: Store -> IO ()
close (Store c) = S.close c

type AlertRow = (String, String, String, String, String, String, String, String, Maybe String)

toAlert :: AlertRow -> Maybe Alert
toAlert (i, s, h, sv, us, hl, ar, se, u) = do
  hz <- lookup h [(show x, x) | x <- [minBound .. maxBound]]
  sev <- lookup sv [(show x, x) | x <- allSeverities]
  uf <- traverse parseUfExact (if null us then [] else splitComma us)
  pure (Alert i s hz sev uf hl ar se u)
  where
    splitComma t = case break (== ',') t of
      (a, []) -> [a]
      (a, _ : r) -> a : splitComma r

-- | The stored alert that a source gave with an id, if any.
findAlert :: Store -> String -> String -> IO (Maybe Alert)
findAlert (Store c) src i = do
  rows <- query c "SELECT alert_id, source, hazard, severity, ufs, headline, area, sent, url FROM alerts WHERE source = ? AND alert_id = ?" (src, i)
  pure (case rows of r : _ -> toAlert r; [] -> Nothing)

-- | Stores an alert from a source. The same (source, id) updates the stored alert, so a source
-- that reads the same alert twice keeps one record.
recordAlert :: Store -> Alert -> IO ()
recordAlert (Store c) a =
  execute
    c
    "INSERT INTO alerts (alert_id, source, hazard, severity, ufs, headline, area, sent, url)\
    \ VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)\
    \ ON CONFLICT (source, alert_id) DO UPDATE SET\
    \ hazard = excluded.hazard, severity = excluded.severity, ufs = excluded.ufs,\
    \ headline = excluded.headline, area = excluded.area, sent = excluded.sent, url = excluded.url,\
    \ updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')"
    ( (alertId a, source a, show (hazard a), show (severity a), intercalate "," (map ufText (ufs a)))
        :. (headline a, area a, sent a, url a)
    )

-- | Every stored alert, oldest first.
listAlerts :: Store -> IO [Alert]
listAlerts (Store c) =
  mapMaybe toAlert <$> query_ c "SELECT alert_id, source, hazard, severity, ufs, headline, area, sent, url FROM alerts ORDER BY id"

data FeedRow = FeedRow
  { feedSlug :: String,
    feedRowTitle :: String,
    feedUrl :: String,
    feedEnabled :: Bool
  }
  deriving (Show, Eq)

addFeed :: Store -> FeedRow -> IO ()
addFeed (Store c) f =
  execute
    c
    "INSERT INTO feeds (slug, title, url, enabled) VALUES (?, ?, ?, ?)\
    \ ON CONFLICT (slug) DO UPDATE SET title = excluded.title, url = excluded.url, enabled = excluded.enabled"
    (feedSlug f, feedRowTitle f, feedUrl f, feedEnabled f)

listFeeds :: Store -> IO [FeedRow]
listFeeds (Store c) =
  map (\(s, t, u, e) -> FeedRow s t u e) <$> query_ c "SELECT slug, title, url, enabled FROM feeds ORDER BY id"

-- | Stores the items that the fetcher read from a feed, in one transaction. The same (feed, key)
-- updates the stored item. Returns how many items it wrote.
recordItems :: Store -> String -> [Item] -> IO Int
recordItems (Store c) feed is = withTransaction c $ do
  mapM_
    ( \i ->
        execute
          c
          "INSERT INTO feed_items (feed, key, title, link, summary, updated) VALUES (?, ?, ?, ?, ?, ?)\
          \ ON CONFLICT (feed, key) DO UPDATE SET title = excluded.title, link = excluded.link,\
          \ summary = excluded.summary, updated = excluded.updated"
          (feed, itemKey i, itemTitle i, itemLink i, itemSummary i, itemUpdated i)
    )
    is
  pure (length is)

-- | The stored items of one feed.
itemsOf :: Store -> String -> IO [Item]
itemsOf (Store c) feed =
  map (\(k, t, l, s, u) -> Item feed k t l s u Nothing)
    <$> query c "SELECT key, title, link, summary, updated FROM feed_items WHERE feed = ? ORDER BY id" (Only feed)

data RuleRow = RuleRow
  { ruleSlug :: String,
    ruleTitle :: String,
    ruleExpr :: Expr
  }
  deriving (Show, Eq)

addRule :: Store -> RuleRow -> IO ()
addRule (Store c) r =
  execute
    c
    "INSERT INTO rules (slug, title, expr) VALUES (?, ?, ?)\
    \ ON CONFLICT (slug) DO UPDATE SET title = excluded.title, expr = excluded.expr"
    (ruleSlug r, ruleTitle r, TE.decodeUtf8 (BL.toStrict (encode (ruleExpr r))))

-- | Every stored rule. A rule whose JSON no longer parses is an error, never a silent drop.
listRules :: Store -> IO (Either String [RuleRow])
listRules (Store c) = do
  rows <- query_ c "SELECT slug, title, expr FROM rules ORDER BY id"
  pure $ traverse (\(s, t, e) -> either (\m -> Left ("rule " <> s <> ": " <> m)) (Right . RuleRow s t) (eitherDecodeStrict (TE.encodeUtf8 (T.pack e)))) rows

findRule :: Store -> String -> IO (Either String (Maybe RuleRow))
findRule s slug = fmap (lookup slug . map (\r -> (ruleSlug r, r))) <$> listRules s
