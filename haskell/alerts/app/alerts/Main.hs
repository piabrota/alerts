-- | The alerts service: publishes alerts to Discord and to an Atom + CAP 1.2 feed, and reads
-- the RSS feeds that the rules compose.
module Main (main) where

import Alerts.Catalog
import Alerts.Core (userAgent)
import Alerts.Discord qualified as Discord
import Alerts.Env
import Alerts.Feed (FeedError (..), FeedMeta (..), Status (..), atom)
import Alerts.Http
import Alerts.Rss.Parse (parse)
import Alerts.Service
import Alerts.Store
import Alerts.Tokens
import Control.Monad (forM, forM_, unless, when)
import Data.Aeson (eitherDecode)
import Data.List (intercalate, isPrefixOf)
import Data.Maybe (fromMaybe, isNothing)
import Data.Text.Lazy qualified as TL
import Data.Text.Lazy.Encoding qualified as TLE
import System.Environment (getArgs)
import System.Exit (ExitCode (..), exitFailure, exitWith)
import System.IO (hPutStrLn, stderr)

usage :: String
usage =
  unlines
    [ "usage: alerts test-message | marola-discord | poll [<source>...] | sources [<filter>...] | tokens",
      "              | feed-add <slug> <url> [<title>] | rule-add <slug> <title> <json>",
      "              | rss-fetch | rss-atom <rule> | version",
      "",
      "Publishes to every configured channel:",
      "  DISCORD_WEBHOOK_URL   post to this Discord webhook",
      "  FEED_PATH             write an Atom + CAP 1.2 feed to this file",
      "  FEED_ID               the feed id URI (default: urn:piabrota-alerts)",
      "",
      "Reads the sources and stores what it delivered:",
      "  ALERTS_DB             the database file (default: alerts.db)",
      "  MOCK_ALERTS_PATH      a JSON file of mock alerts, in place of the built-in scenarios",
      "  ALERTS_SOURCES        the sources that `poll` reads when it gets none, for example cemaden",
      "  ALERTS_UFS            the states that `poll` delivers: all, or for example PR,SC,RS",
      "                        (default: PR,SC,RS)",
      "  ALERTS_TOKEN_<ID>     BYOT: your own token for a catalog source that needs one",
      "",
      "Every variable can also come from a .env file in the current directory or a parent; a",
      "variable that is already set wins, and an empty one counts as not set.",
      "",
      "  test-message     publish one test alert (CAP status Test)",
      "  marola-discord   post one test alert to MAROLA_DISCORD_WEBHOOK_URL (the hard rule of every PR)",
      "  poll <source>... read each source once, keep the alerts of ALERTS_UFS, and deliver",
      "                   each new alert once (sources: mock, cemaden; default: ALERTS_SOURCES)",
      "  sources [...]    search the curated catalog of official sources, in Brazil and in the",
      "                   world: words, --country <ISO code>, --no-token, --alerts",
      "  tokens           say which BYOT tokens are set (never their values)",
      "  feed-add         store an RSS or Atom feed that rules can read",
      "  rule-add         store a rule (the JSON of the rule algebra, docs/RSS.md)",
      "  rss-fetch        read every enabled feed and store its items",
      "  rss-atom <rule>  print the Atom feed of the rule with this slug"
    ]

die' :: String -> IO a
die' msg = hPutStrLn stderr msg >> exitFailure

orDie :: Either String a -> IO a
orDie = either die' pure

usageExit :: IO a
usageExit = hPutStrLn stderr usage >> exitWith (ExitFailure 2)

main :: IO ()
main = do
  loadDotenv >>= orDie
  args <- getArgs
  case args of
    ["test-message"] -> testMessage
    ["marola-discord"] -> marolaDiscord
    "poll" : ids -> poll ids
    "sources" : rest -> sources rest
    ["tokens"] -> forM_ byot $ \(e, _) -> do
      st <- tokenState e env
      putStrLn (pad 16 (entryId e) <> " " <> describe st)
    ["feed-add", slug, u] -> withDb (\db -> addFeed db (FeedRow slug slug u True)) >> putStrLn ("feed " <> slug <> " stored")
    ["feed-add", slug, u, title] -> withDb (\db -> addFeed db (FeedRow slug title u True)) >> putStrLn ("feed " <> slug <> " stored")
    ["rule-add", slug, title, json] -> do
      expr <- orDie (either (Left . ("invalid rule: " <>)) Right (eitherDecode (TLE.encodeUtf8 (TL.pack json))))
      withDb $ \db -> do
        let row = RuleRow slug title expr
        checkRule db row >>= orDie
        addRule db row
      putStrLn ("rule " <> slug <> " stored")
    ["rss-fetch"] -> rssFetch
    ["rss-atom", slug] -> withDb (\db -> ruleAtom db slug) >>= orDie >>= putStr
    ["version"] -> putStrLn userAgent
    _ -> usageExit
  where
    pad n s = s <> replicate (n - length s) ' '

withDb :: (Store -> IO a) -> IO a
withDb f = do
  path <- fromMaybe "alerts.db" <$> env "ALERTS_DB"
  db <- open path
  r <- f db
  close db
  pure r

feedMeta :: IO FeedMeta
feedMeta = do
  i <- fromMaybe "urn:piabrota-alerts" <$> env "FEED_ID"
  pure (FeedMeta i "alerts")

testMessage :: IO ()
testMessage = do
  now <- nowUtc
  let a = testAlert now
  discord <- env "DISCORD_WEBHOOK_URL"
  feed <- env "FEED_PATH"
  when (isNothing discord && isNothing feed) $ die' "no channel: set DISCORD_WEBHOOK_URL or FEED_PATH"
  h <- newHttp
  forM_ discord $ \u -> do
    i <- Discord.post h u a >>= orDie
    putStrLn ("discord: test message sent (message " <> i <> ")")
  forM_ feed $ \path -> do
    meta <- feedMeta
    xml <- orDie (either (\(InvalidTime t) -> Left ("invalid CAP time: " <> show t)) Right (atom meta [a] Test now))
    writeAtomic path xml
    putStrLn ("feed: test entry written to " <> path)

-- | The hard rule of every PR: one test alert reaches the official Marola Discord. The webhook
-- comes from MAROLA_DISCORD_WEBHOOK_URL (the environment or .env) and never enters the
-- repository; it is never printed.
marolaDiscord :: IO ()
marolaDiscord = do
  hook <- env "MAROLA_DISCORD_WEBHOOK_URL"
  u <- maybe (die' "set MAROLA_DISCORD_WEBHOOK_URL to the webhook of the official Marola Discord") pure hook
  unless (Discord.isDiscordWebhook u) $ die' "MAROLA_DISCORD_WEBHOOK_URL is not a Discord webhook URL"
  now <- nowUtc
  h <- newHttp
  r <- Discord.post h u (testAlert now)
  case r of
    Left e -> die' ("the Marola Discord did not store the test alert: " <> e)
    Right i -> putStrLn ("marola-discord: test alert stored as message " <> i)

-- | Reads each source once and delivers its new alerts to every configured channel. A source
-- that fails is reported, and the others still run; the command fails when any source failed.
poll :: [String] -> IO ()
poll given = do
  subscribed <- maybe [] (words . map (\c -> if c == ',' then ' ' else c)) <$> env "ALERTS_SOURCES"
  let ids = if null given then subscribed else given
  when (null ids) usageExit
  srcs <- orDie (traverse parseSource ids)
  discord <- env "DISCORD_WEBHOOK_URL"
  feedPath <- env "FEED_PATH"
  meta <- feedMeta
  let channels = Channels discord ((\p -> (p, meta)) <$> feedPath)
  when (isNothing discord && isNothing feedPath) $ die' "no channel: set DISCORD_WEBHOOK_URL or FEED_PATH"
  hs <- orDie . fmap harness . ufsOf =<< env "ALERTS_UFS"
  now <- nowUtc
  mockFile <- env "MOCK_ALERTS_PATH"
  h <- newHttp
  withDb $ \db -> do
    failures <- forM srcs $ \src -> do
      r <- fetch h (Fetch now mockFile) src >>= either (pure . Left) (deliver h db hs channels src)
      case r of
        Right rep -> do
          putStrLn (sourceIdOf src <> ": " <> show (fetched rep) <> " read, " <> show (passed rep) <> " passed the harness, " <> show (delivered rep) <> " delivered")
          pure 0
        Left e -> hPutStrLn stderr (sourceIdOf src <> ": " <> e) >> pure (1 :: Int)
    forM_ feedPath $ \path -> do
      n <- writeFeed db path meta now >>= orDie
      putStrLn ("feed: " <> show n <> " entries written to " <> path)
    let failed = sum failures
    when (failed > 0) $ die' (show failed <> " sources failed")

-- | Prints the catalog entries that match the filters: one block per entry, with its token state.
sources :: [String] -> IO ()
sources = go emptyQuery []
  where
    go q ws args = case args of
      "--country" : c : rest -> go q {qCountry = Just c} ws rest
      ["--country"] -> die' "--country needs an ISO code"
      "--no-token" : rest -> go q {qNoToken = True} ws rest
      "--alerts" : rest -> go q {qAlertsOnly = True} ws rest
      a : _ | "--" `isPrefixOf` a -> die' ("unknown option " <> a)
      a : rest -> go q (ws <> [a]) rest
      [] -> printAll (search q {qText = unwords ws})
    printAll found = do
      forM_ found $ \e -> do
        let countries = if null (entryCountries e) then "world" else unwords (entryCountries e)
            kind = case entryKind e of AlertsKind -> "alerts"; Observations -> "observations"
            modl = maybe "catalog only" ("alerts poll " <>) (entryModule e)
        st <- tokenState e env
        putStrLn (entryId e <> " · " <> entryAgency e <> " · " <> entryName e)
        putStrLn ("  " <> countries <> " · " <> kind <> " · " <> entryFormat e <> " · " <> intercalate ", " (entryHazards e))
        putStrLn ("  " <> entryUrl e)
        forM_ (entryDocs e) $ \d -> putStrLn ("  docs: " <> d)
        putStrLn ("  " <> describe st <> " · " <> modl)
        case entryAuth e of
          Token _ how -> putStrLn ("  get a token: " <> how)
          NoAuth -> pure ()
        putStrLn ("  checked: " <> entryChecked e)
      putStrLn (show (length found) <> " of " <> show (length catalog) <> " sources")

-- | Reads each enabled feed once and stores its items. A feed that fails is reported, and the
-- others still run; the command fails when any feed failed.
rssFetch :: IO ()
rssFetch = withDb $ \db -> do
  h <- newHttp
  feeds <- filter feedEnabled <$> listFeeds db
  failures <- forM feeds $ \f -> do
    body <- get h (feedUrl f)
    case body >>= \b -> either (Left . show) Right (parse (TL.unpack (TLE.decodeUtf8With (\_ _ -> Just '\xFFFD') b)) (feedSlug f)) of
      Left e -> hPutStrLn stderr (feedSlug f <> ": " <> e) >> pure (1 :: Int)
      Right is -> do
        n <- recordItems db (feedSlug f) is
        putStrLn (feedSlug f <> ": " <> show n <> " items")
        pure 0
  let failed = sum failures
  when (failed > 0) $ die' (show failed <> " feeds failed")
