-- | The sources of the service and the poll that joins them to the channels.
--
-- A source module turns the data of one agency into alerts. 'Source' is the router: a pure match
-- on an enum, with no class hierarchy. A new source adds one constructor and one case.
module Alerts.Service
  ( Source (..),
    allSources,
    sourceIdOf,
    parseSource,
    statusOf,
    sourceStatus,
    Fetch (..),
    fetch,
    harness,
    v0Harness,
    ufsOf,
    Channels (..),
    noChannels,
    Report (..),
    deliver,
    writeFeed,
    testAlert,
    nowUtc,
    writeAtomic,
    ruleItems,
    ruleAtom,
    checkRule,
  )
where

import Alerts.Cap (identifier)
import Alerts.Core hiding (deliver)
import Alerts.Discord qualified as Discord
import Alerts.Feed
import Alerts.Http
import Alerts.Rss.Expr
import Alerts.Rss.Item
import Alerts.Source.Cemaden qualified as Cemaden
import Alerts.Source.Mock qualified as Mock
import Alerts.Store
import Alerts.Time (rfc3339Utc)
import Control.Monad (foldM)
import Data.ByteString.Lazy qualified as BL
import Data.Char (toLower)
import Data.List (find, intercalate)
import Data.Map.Strict qualified as Map
import Data.Maybe (isJust, mapMaybe)
import Data.Set qualified as Set
import Data.Time.Clock.POSIX (getPOSIXTime)
import System.Directory (renameFile)

-- | One source of alerts.
data Source
  = -- | Simulated test alerts ("Alerts.Source.Mock"). Never an agency.
    Mock
  | -- | The municipal alerts of the CEMADEN panel ("Alerts.Source.Cemaden").
    Cemaden
  deriving (Show, Eq, Enum, Bounded)

allSources :: [Source]
allSources = [minBound .. maxBound]

sourceIdOf :: Source -> String
sourceIdOf Mock = Mock.sourceId
sourceIdOf Cemaden = Cemaden.sourceId

parseSource :: String -> Either String Source
parseSource s = case find ((== s) . sourceIdOf) allSources of
  Just src -> Right src
  Nothing -> Left ("unknown source " <> show s <> "; the sources are: " <> intercalate ", " (map sourceIdOf allSources))

-- | The CAP status of the alerts of a source. A mock alert is always a test.
sourceStatus :: Source -> Status
sourceStatus Mock = Test
sourceStatus Cemaden = Actual

-- | The CAP status of a stored alert. A source that this build does not know is published as a
-- test: a test alert must never be published as 'Actual'.
statusOf :: String -> Status
statusOf s = either (const Test) sourceStatus (parseSource s)

-- | What a source may need to read its alerts.
data Fetch = Fetch
  { -- | The time of this poll, in the form of 'sent'.
    fetchNow :: String,
    -- | A mock file (@MOCK_ALERTS_PATH@) in place of the built-in mock scenarios.
    fetchMockFile :: Maybe FilePath
  }

-- | Reads the alerts that the source states now, in the order of the source.
fetch :: Http -> Fetch -> Source -> IO (Either String [Alert])
fetch h ctx src = case src of
  Mock -> case fetchMockFile ctx of
    Nothing -> pure (Right (Mock.scenarios (fetchNow ctx)))
    Just path -> do
      body <- BL.readFile path
      pure (either (Left . Mock.describeMockError) Right (Mock.parse body (fetchNow ctx)))
  Cemaden -> do
    r <- get h Cemaden.panelUrl
    pure (r >>= either (Left . Cemaden.describeCemadenError) Right . Cemaden.parse)

-- | The harness that keeps the alerts of these states.
harness :: [Uf] -> Harness
harness us = stages [onlyUfs us]

-- | The harness of v0: floods in PR, SC and RS.
v0Harness :: Harness
v0Harness = harness v0

-- | The states of @ALERTS_UFS@: @all@ (every UF), or UFs separated by commas or spaces, for
-- example @PR,SC,RS@. Not set: the states of v0.
ufsOf :: Maybe String -> Either String [Uf]
ufsOf Nothing = Right v0
ufsOf (Just v)
  | map toLower (trim v) == "all" = Right allUfs
  | otherwise = case words (map (\c -> if c == ',' then ' ' else c) v) of
      [] -> Right v0
      ws -> traverse (\w -> maybe (Left ("unknown UF: " <> show w)) Right (parseUf w)) ws

-- | The channels of one poll. Each one is optional, but a poll needs at least one.
data Channels = Channels
  { chDiscord :: Maybe String,
    chFeed :: Maybe (FilePath, FeedMeta)
  }

noChannels :: Channels
noChannels = Channels Nothing Nothing

-- | What one source gave in one poll.
data Report = Report {fetched, passed, delivered :: Int}
  deriving (Show, Eq)

-- | Delivers the new alerts of one source: each alert that passes the harness and that the store
-- does not have yet goes to Discord, then into the store. An alert is stored only after Discord
-- stored it, so a failed delivery is tried again at the next poll.
deliver :: Http -> Store -> Harness -> Channels -> Source -> [Alert] -> IO (Either String Report)
deliver h db hs ch src as = go (Report (length as) 0 0) (mapMaybe (run hs) as)
  where
    go r [] = pure (Right r)
    go r (a : rest)
      | source a /= sourceIdOf src =
          pure (Left (sourceIdOf src <> ": alert " <> alertId a <> " names the source " <> show (source a)))
      | otherwise = do
          let r' = r {passed = passed r + 1}
          old <- findAlert db (source a) (alertId a)
          if isJust old
            then go r' rest
            else do
              posted <- maybe (pure (Right "")) (\u -> Discord.post h u a) (chDiscord ch)
              case posted of
                Left e -> pure (Left e)
                Right _ -> do
                  recordAlert db a
                  go r' {delivered = delivered r' + 1} rest

-- | Writes the feed of every stored alert, newest first. The feed and Discord read the same
-- store, so every alert that reached Discord is in the feed, with the same text.
writeFeed :: Store -> FilePath -> FeedMeta -> String -> IO (Either String Int)
writeFeed db path meta now = do
  as <- reverse <$> listAlerts db
  let entries =
        [ Entry
            { entryId = feedId meta <> ":" <> identifier a,
              entryTitle = headline a,
              entryLink = url a,
              entryUpdated = sent a,
              entrySummary = area a,
              entryAlert = Just (a, statusOf (source a))
            }
        | a <- as
        ]
  case atomEntries meta entries now of
    Left (InvalidTime t) -> pure (Left ("invalid CAP time: " <> show t))
    Right xml -> writeAtomic path xml >> pure (Right (length entries))

testAlert :: String -> Alert
testAlert t =
  Alert
    { alertId = "alerts-test",
      source = "alerts",
      hazard = Flood,
      severity = Minor,
      ufs = v0,
      headline = "TESTE: esta mensagem não é um alerta real.",
      area = "O alerts enviou esta mensagem para testar os canais de publicação.",
      sent = t,
      url = Nothing
    }

-- | The current time as @YYYY-MM-DDThh:mm:ss+00:00@, the form CAP 1.2 and Atom accept.
nowUtc :: IO String
nowUtc = rfc3339Utc . floor <$> getPOSIXTime

-- | Readers never see a half-written feed: write a sibling file, then rename it.
writeAtomic :: FilePath -> String -> IO ()
writeAtomic path contents = do
  let tmp = path <> ".tmp"
  writeFile tmp contents
  renameFile tmp path

-- | The environment that a set of rules reads: every feed (a feed with no stored item is still
-- a feed), the stored alerts when a rule reads them, and the rules.
envFor :: Store -> [RuleRow] -> IO Env
envFor db rules = do
  feeds <- listFeeds db
  let rs = Map.fromList [(ruleSlug r, ruleExpr r) | r <- rules]
      reached = foldMap (refFeeds . refs) (Map.elems rs)
      wantsAlerts = any (refAlerts . refs) (Map.elems rs)
  items <- foldM (\m f -> (\is -> Map.insert f is m) <$> itemsOf db f) Map.empty (Set.toList reached)
  let fs = Map.union items (Map.fromList [(feedSlug f, []) | f <- feeds])
  as <- if wantsAlerts then map fromAlert <$> listAlerts db else pure []
  pure (Env fs as rs)

-- | The items of the rule with a slug, from the stored items of its feeds and the stored alerts.
ruleItems :: Store -> String -> IO (Either String (RuleRow, [Item]))
ruleItems db slug = do
  rules <- listRules db
  case rules of
    Left e -> pure (Left e)
    Right rs -> case find ((== slug) . ruleSlug) rs of
      Nothing -> pure (Left ("no rule " <> slug))
      Just r -> do
        e <- envFor db rs
        pure (either (Left . describeEvalError) (\is -> Right (r, is)) (eval (Rule slug) e))

-- | The rule as an Atom feed, through the Atom + CAP 1.2 channel. The feed time is the time of
-- its newest item.
ruleAtom :: Store -> String -> IO (Either String String)
ruleAtom db slug = do
  r <- ruleItems db slug
  pure $ do
    (rule, is) <- r
    let meta = FeedMeta ("urn:piabrota-alerts:rule:" <> ruleSlug rule) (ruleTitle rule)
        updated = case mapMaybe itemUpdated is of
          [] -> "1970-01-01T00:00:00+00:00"
          ts -> maximum ts
    either (\(InvalidTime t) -> Left ("invalid CAP time: " <> show t)) Right (atomOfItems meta is updated)

-- | Evaluates a new rule on empty feeds: this finds an unknown feed or rule, a bad time, and a
-- rule that reaches itself, before anything is stored.
checkRule :: Store -> RuleRow -> IO (Either String ())
checkRule db new = do
  rules <- listRules db
  feeds <- listFeeds db
  pure $ do
    rs <- rules
    let others = Map.fromList [(ruleSlug r, ruleExpr r) | r <- rs, ruleSlug r /= ruleSlug new]
        e = Env (Map.fromList [(feedSlug f, []) | f <- feeds]) [] (Map.insert (ruleSlug new) (ruleExpr new) others)
    either (\m -> Left ("A regra não pode ser avaliada: " <> describeEvalError m <> ".")) (const (Right ())) (eval (Rule (ruleSlug new)) e)
