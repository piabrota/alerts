{-# LANGUAGE OverloadedStrings #-}

-- | The tests of the alerts service: every test of the previous implementation, and the
-- counterexamples W1 to W6 (docs/PROOFS.md), replayed on the Haskell code.
module Main (main) where

import Alerts.Cap (identifier)
import Alerts.Catalog qualified as Cat
import Alerts.Core hiding (deliver)
import Alerts.Discord (isDiscordWebhook, payload)
import Alerts.Env (parseDotenv, readableByOthers)
import Alerts.Feed
import Alerts.Http (newHttp)
import Alerts.Rss.Expr
import Alerts.Rss.Item
import Alerts.Rss.Parse qualified as P
import Alerts.Service
import Alerts.Source.Cemaden qualified as Cemaden
import Alerts.Source.Mock qualified as Mock
import Alerts.Store
import Alerts.Time
import Alerts.Tokens
import Control.Monad (unless)
import Data.Aeson (Value (..), decode, eitherDecode, encode, object, (.=))
import Data.Aeson.KeyMap qualified as KM
import Data.ByteString.Lazy qualified as BL
import Data.ByteString.Lazy.Char8 qualified as BLC
import Data.Either (isLeft, isRight)
import Data.Functor.Identity (Identity (..))
import Data.IORef
import Data.List (elemIndex, isInfixOf, isPrefixOf, nub, sort)
import Data.Map.Strict qualified as Map
import Data.Maybe (isJust, isNothing, mapMaybe)
import Data.Set qualified as Set
import Data.Text qualified as T
import Data.Text.Lazy qualified as TL
import Data.Text.Lazy.Encoding qualified as TLE
import System.Directory (getTemporaryDirectory, removeFile)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import Text.XML qualified as X

type Check = String -> Bool -> IO ()

main :: IO ()
main = do
  failures <- newIORef (0 :: Int)
  let check name ok = unless ok $ do
        putStrLn ("FAIL " <> name)
        modifyIORef failures (+ 1)
  timeTests check
  feedTests check
  parseTests check
  exprTests check
  witnesses check
  catalogTests check
  cemadenTests check
  mockTests check
  tokenTests check
  envTests check
  serviceTests check
  n <- readIORef failures
  if n == 0 then putStrLn "alerts: every test passes" else putStrLn (show n <> " failed") >> exitFailure

utf8 :: String -> BL.ByteString
utf8 = TLE.encodeUtf8 . TL.pack

------------------------------------------------------------------------

timeTests :: Check -> IO ()
timeTests check = do
  check "known_dates" $
    rfc3339Utc 0 == "1970-01-01T00:00:00+00:00"
      && rfc3339Utc 951782400 == "2000-02-29T00:00:00+00:00"
      && rfc3339Utc 1792627200 == "2026-10-22T00:00:00+00:00"
      && daysFromCivil 2000 2 29 * 86400 == 951782400
  let utc = Just "2026-10-22T15:00:00+00:00"
  check "both_forms_become_utc" $
    all
      ((== utc) . normalize)
      [ "2026-10-22T12:00:00-03:00",
        "2026-10-22T15:00:00Z",
        "2026-10-22T15:00:00.123Z",
        "Thu, 22 Oct 2026 15:00:00 GMT",
        "22 Oct 2026 12:00:00 -0300",
        "Thu, 22 Oct 26 11:00 EDT"
      ]
  check "bad_times_are_none" $
    all (isNothing . normalize) ["", "ontem", "2026-13-01T00:00:00Z", "Thu, 22 Foo 2026 15:00:00 GMT", "2026-10-22T15:00:00"]
  -- the calendar is exact for every day of 1900 to 2100 (P4 civil_round_trip)
  check "civil_round_trip" $
    and [rfc3339Utc (d * 86400) == civil | d <- [-25567, -25500 .. 47482], let civil = rfc3339Utc (d * 86400), normalize civil == Just civil]

------------------------------------------------------------------------

sample :: Alert
sample =
  Alert
    { alertId = "a 1,<x>",
      source = "inmet",
      hazard = Flood,
      severity = Severe,
      ufs = [RS, SC],
      headline = "Risco de inundação em Porto Alegre & região.",
      area = "Porto Alegre (RS)",
      sent = "2026-10-22T12:00:00-03:00",
      url = Just "https://example.org/a?b=1&c=2"
    }

-- | Every element name in document order, or Nothing on malformed XML.
names :: String -> Maybe [String]
names xml = case X.parseText X.def (TL.pack xml) of
  Left _ -> Nothing
  Right d -> Just (go (X.documentRoot d))
  where
    go e = T.unpack (X.nameLocalName (X.elementName e)) : concat [go c | X.NodeElement c <- X.elementNodes e]

feedTests :: Check -> IO ()
feedTests check = do
  let Right x = cap sample Actual
      order = ["alert", "identifier", "sender", "sent", "status", "msgType", "source", "scope", "info", "language", "category", "event", "urgency", "severity", "certainty", "headline", "web", "area", "areaDesc", "geocode"]
      ns = maybe [] id (names x)
      pos = map (`elemIndex` ns) order
  check "cap_has_required_elements_in_schema_order" $
    all isJust pos
      && and (zipWith (<) pos (drop 1 pos))
      && "<identifier>piabrota-alerts-inmet-a_1__x_</identifier>" `isInfixOf` x
      && "&amp; região" `isInfixOf` x
      && length (filter (== "geocode") ns) == 2
  check "test_status_is_kept" (either (const False) ("<status>Test</status>" `isInfixOf`) (cap sample Test))
  check "invalid_time_is_rejected" $
    isLeft (cap sample {sent = "2026-10-22 12:00"} Actual) && isLeft (cap sample {sent = ""} Actual)
  let meta = FeedMeta "urn:piabrota-alerts" "alerts"
      Right a = atom meta [sample, sample] Actual "2026-10-22T15:00:00+00:00"
      an = maybe [] id (names a)
  check "atom_is_well_formed_with_inline_cap" $
    isJust (names a)
      && length (filter (== "entry") an) == 2
      && length (filter (== "alert") an) == 2
      && "<content type=\"application/cap+xml\"><alert xmlns=" `isInfixOf` a

------------------------------------------------------------------------

rss :: String
rss =
  "<?xml version=\"1.0\"?>\n\
  \<rss version=\"2.0\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n\
  \  <channel><title>Notícias</title><link>https://example.org/</link>\n\
  \    <item>\n\
  \      <title>Chuva forte &amp; alagamento</title>\n\
  \      <link>https://example.org/a</link>\n\
  \      <guid isPermaLink=\"false\">a-1</guid>\n\
  \      <description><![CDATA[<p>Rio sobe em <b>Blumenau</b>.</p>]]></description>\n\
  \      <pubDate>Thu, 22 Oct 2026 15:00:00 GMT</pubDate>\n\
  \    </item>\n\
  \    <item><guid>b-2</guid><title>Sem link</title><content:encoded>Corpo</content:encoded></item>\n\
  \    <item><description>Sem identidade</description></item>\n\
  \  </channel>\n\
  \</rss>"

atomDoc :: String
atomDoc =
  "<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>t</title>\n\
  \  <entry><id>urn:x:1</id><title type=\"text\">Primeiro</title>\n\
  \    <link rel=\"self\" href=\"https://example.org/self\"/>\n\
  \    <link href=\"https://example.org/1\"/>\n\
  \    <updated>2026-10-22T12:00:00-03:00</updated><summary>Resumo &#233;</summary>\n\
  \    <author><name>Não é título</name></author>\n\
  \  </entry>\n\
  \</feed>"

parseTests :: Check -> IO ()
parseTests check = do
  case P.parse rss "g1" of
    Right [a, b] ->
      check "rss_items" $
        itemFeed a == "g1"
          && itemKey a == "https://example.org/a"
          && itemTitle a == "Chuva forte & alagamento"
          && itemSummary a == "<p>Rio sobe em <b>Blumenau</b>.</p>"
          && itemUpdated a == Just "2026-10-22T15:00:00+00:00"
          && itemKey b == "b-2"
          && itemSummary b == "Corpo"
          && isNothing (itemUpdated b)
    other -> check ("rss_items: " <> show other) False
  case P.parse atomDoc "g2" of
    Right [e] ->
      check "atom_entries" $
        itemKey e == "https://example.org/1"
          && itemTitle e == "Primeiro"
          && itemSummary e == "Resumo é"
          && itemUpdated e == Just "2026-10-22T15:00:00+00:00"
    other -> check ("atom_entries: " <> show other) False
  check "not_a_feed" $
    P.parse "<html><body/></html>" "x" == Left (P.NotAFeed "html") && isLeft (P.parse "" "x")

------------------------------------------------------------------------

item :: String -> String -> String -> Maybe String -> Item
item f k ti u = Item f k ti Nothing ("<p>Resumo de " <> ti <> "</p>") u Nothing

alertItem :: String -> Uf -> Severity -> Item
alertItem i u s =
  fromAlert
    Alert
      { alertId = i,
        source = "defesa-civil-idap",
        hazard = Flood,
        severity = s,
        ufs = [u],
        headline = "Alerta " <> i,
        area = "Blumenau (SC)",
        sent = "2026-10-22T12:00:00-03:00",
        url = Nothing
      }

t :: Int -> Maybe String
t h = Just ("2026-10-22T" <> (if h < 10 then "0" else "") <> show h <> ":00:00+00:00")

env0 :: Env
env0 =
  Env
    { envFeeds =
        Map.fromList
          [ ("g1", [item "g1" "a" "Chuva em Blumenau" (t 10), item "g1" "b" "Enchente no Vale" (t 12), item "g1" "c" "Futebol" Nothing]),
            ("uol", [item "uol" "b" "Enchente no Vale do Itajaí" Nothing, item "uol" "d" "CHUVA  em blumenau" (t 9)]),
            ("vazio", [])
          ],
      envAlerts = [alertItem "1" SC Severe, alertItem "2" SP Minor],
      envRules = Map.empty
    }

keys :: Expr -> Env -> [String]
keys e en = either (const ["<error>"]) (map itemKey) (eval e en)

exprTests :: Check -> IO ()
exprTests check = do
  let json = "{\"op\": \"take\", \"n\": 5, \"of\": {\"op\": \"union\", \"of\": [{\"op\": \"feed\", \"feed\": \"g1\"}, {\"op\": \"filter\", \"when\": {\"is\": \"min_severity\", \"severity\": \"Severe\"}, \"of\": {\"op\": \"alerts\"}}]}}"
      e = eitherDecode (utf8 json) :: Either String Expr
  check "json_form" $
    isRight e
      && (eitherDecode . encode =<< e) == e
      && isLeft (eitherDecode "{\"op\": \"feed\", \"feed\": \"g1\", \"x\": 1}" :: Either String Expr)
      && isLeft (eitherDecode "{\"op\": \"zip\"}" :: Either String Expr)

  let u = Union [Feed "g1", Feed "uol"]
  check "union_is_a_set_union" $
    keys u env0 == ["b", "a", "d", "c"]
      && (fmap (itemFeed . head) (eval u env0) == Right "g1")
      && eval u env0 == eval (Union [Feed "uol", Feed "g1"]) env0
      && null (keys (Union []) env0)

  let (a, b, c) = (Feed "g1", Feed "uol", Alerts)
      same x y = eval x env0 == eval y env0
  check "set_laws" $
    and
      [ same (op [a, b]) (op [b, a]) && same (op [op [a, b], c]) (op [a, op [b, c]]) && same (op [a, a]) a
      | op <- [Union, Intersect]
      ]
      && keys (Intersect [a, b]) env0 == ["b"]
      && null (keys (Diff a a) env0)
      && keys (Diff a b) env0 == ["a", "c"]

  let p = Contains TitleField "chuva"
      q = Since "2026-10-22T09:30:00Z"
      both = Union [Feed "g1", Feed "uol"]
  check "filter_laws" $
    eval (Filter p (Filter q both)) env0 == eval (Filter (All [p, q]) both) env0
      && eval (Filter p both) env0 == eval (Union [Filter p (Feed "g1"), Filter p (Feed "uol")]) env0
      && keys (Filter p both) env0 == ["a", "d"]

  let severe = eval (Filter (MinSeverity Severe) (Union [Alerts, Feed "g1"])) env0
  check "alert_predicates_reuse_the_harness" $
    fmap (map (fmap alertId . itemAlert)) severe == Right [Just "1"]
      && fmap length (eval (Filter (Ufs v0) Alerts) env0) == Right 1
      && keys (Filter (Not (From [alertsFeed])) (Union [Alerts, Feed "uol"])) env0 == ["d", "b"]

  let Right mapped = eval (Map (PrefixTitle "[SC] ") (Union [Feed "g1", Alerts])) env0
      stripped = Map StripHtml (Feed "g1")
      dedupe = Dedupe (Union [Feed "g1", Feed "uol"])
  check "map_dedupe_take" $
    all (("[SC] " `isPrefixOf`) . itemTitle) (filter (isNothing . itemAlert) mapped)
      && all (("Alerta" `isPrefixOf`) . itemTitle) (filter (isJust . itemAlert) mapped)
      && fmap (itemSummary . head) (eval stripped env0) == Right "Resumo de Enchente no Vale"
      && fmap (itemSummary . head) (eval (Map (Truncate 3) stripped) env0) == Right "Res…"
      && keys dedupe env0 == ["b", "a", "c"]
      && keys (Take 2 dedupe) env0 == ["b", "a"]

  let rules =
        Map.fromList
          [ ("chuva", Filter (Contains AnyField "chuva") (Feed "g1")),
            ("tudo", Union [Rule "chuva", Feed "vazio"]),
            ("x", Rule "y"),
            ("y", Rule "x")
          ]
      envR = env0 {envRules = rules}
      r = refs (rules Map.! "tudo")
  check "rules_compose_and_cycles_fail" $
    keys (Rule "tudo") envR == ["a"]
      && eval (Rule "x") envR == Left (Cycle ["x", "y", "x"])
      && eval (Feed "nada") envR == Left (UnknownFeed "nada")
      && eval (Rule "nada") envR == Left (UnknownRule "nada")
      && eval (Filter (Since "ontem") (Feed "vazio")) envR == Left (BadTime "ontem")
      && Set.toList (refFeeds r) == ["vazio"]
      && Set.toList (refRules r) == ["chuva"]
      && not (refAlerts r)
      && refAlerts (refs Alerts)

  let Right is = eval (Union [Alerts, Feed "g1"]) env0
      Right x = atomOfItems (FeedMeta "urn:piabrota-alerts:regra:sul" "Sul") is "2026-10-22T15:00:00+00:00"
      count s = length (filter (s `isPrefixOf`) (tailsOf x))
  check "atom_embeds_cap_for_alert_items" $
    count "<entry>" == 5
      && count "<content type=\"application/cap+xml\">" == 2
      && "<status>Actual</status>" `isInfixOf` x
      && "<id>urn:piabrota-alerts:regra:sul:g1:a</id>" `isInfixOf` x
      && "<id>urn:piabrota-alerts:regra:sul:piabrota-alerts-defesa-civil-idap-1</id>" `isInfixOf` x
      && "<updated>2026-10-22T15:00:00+00:00</updated><summary>&lt;p&gt;Resumo de Futebol" `isInfixOf` x

tailsOf :: [a] -> [[a]]
tailsOf [] = [[]]
tailsOf l@(_ : r) = l : tailsOf r

------------------------------------------------------------------------

-- | The counterexamples of P4 to P6 (docs/PROOFS.md), replayed on the Haskell code: each one is a
-- fact about this code, kept so that a fix shows up as a failing check.
witnesses :: Check -> IO ()
witnesses check = do
  check "W1 accepted_dates_are_dates: February 31 becomes March 3" $
    normalize "2026-02-31T00:00:00Z" == Just "2026-03-03T00:00:00+00:00"
  check "W2 accepted_clock_within_the_day: 23:59:60 is the next midnight" $
    normalize "2026-10-22T23:59:60Z" == normalize "2026-10-23T00:00:00Z" && isJust (normalize "2026-10-22T23:59:60Z")
  check "W3 offset_within_a_day: the offset is not range-checked" $
    normalize "2026-10-22T12:00:00+00:60" == normalize "2026-10-22T12:00:00+01:00" && isJust (normalize "2026-10-22T12:00:00+99:99")
  let far = normalize "10000-01-01T00:00:00Z"
  check "W4 utc_year_has_four_digits: the year 10000 sorts before 2026" $
    far == Just "10000-01-01T00:00:00+00:00" && far < normalize "2026-10-22T00:00:00Z"
  let it f ti = Item f "https://example.org/a" ti Nothing "" Nothing Nothing
      en = emptyEnv {envFeeds = Map.fromList [("g1", [it "g1" "Enchente no Vale"]), ("uol", [it "uol" "Enchente no Vale do Itajaí"])]}
      p = Contains TitleField "itajaí"
      left = eval (Filter p (Union [Feed "g1", Feed "uol"])) en
      right = eval (Union [Filter p (Feed "g1"), Filter p (Feed "uol")]) en
  check "W5 filter_union_distributes: a key clash breaks the law" $
    fmap null left == Right True && fmap length right == Right 1
  let al s i = Alert i s Flood Severe [SC] "Teste." "Teste" "2026-01-01T00:00:00Z" Nothing
  check "W6 identifier_unique: cemaden/ped-1 and cemaden-ped/1 share an identifier" $
    isJust (Cat.getEntry "cemaden")
      && isJust (Cat.getEntry "cemaden-ped")
      && identifier (al "cemaden" "ped-1") == identifier (al "cemaden-ped" "1")
  -- C7: the hypothesis of identifier-unique (spec/Alerts/Properties.agda) over the catalog: no
  -- source id has a '-' before a digit, and none starts or ends with '-'.
  check "C7 catalog ids keep the form of identifier-unique" $
    all
      ( \e ->
          let i = Cat.entryId e
           in not (null i)
                && take 1 i /= "-"
                && take 1 (reverse i) /= "-"
                && not (or (zipWith (\c d -> c == '-' && d `elem` ['0' .. '9']) i (drop 1 i)))
      )
      Cat.catalog

------------------------------------------------------------------------

catalogTests :: Check -> IO ()
catalogTests check = do
  let ids q = map Cat.entryId (Cat.search q)
      q0 = Cat.emptyQuery
  check "ids_are_unique_and_well_formed" $
    let is = map Cat.entryId Cat.catalog
     in nub is == is
          && all (\e -> all (`elem` ('-' : ['a' .. 'z'] <> ['0' .. '9'])) (Cat.entryId e)) Cat.catalog
          && all (("https://" `isPrefixOf`) . Cat.entryUrl) Cat.catalog
          && all (all (\c -> length c == 2 && all (`elem` ['A' .. 'Z']) c) . Cat.entryCountries) Cat.catalog
          && all (\e -> not (null (Cat.entryHazards e)) && not (null (Cat.entryChecked e))) Cat.catalog
  check "every_token_variable_follows_the_rule" $
    all (\(e, v) -> v == Cat.tokenEnv (Cat.entryId e)) Cat.byot && Cat.tokenEnv "meteo-france" == "ALERTS_TOKEN_METEO_FRANCE"
  check "empty_query_finds_everything" (length (Cat.search q0) == length Cat.catalog)
  check "search_by_text_ignores_case_and_accents" $
    ids q0 {Cat.qText = "CEMADEN painel"} == ["cemaden"]
      && "meteo-france" `elem` ids q0 {Cat.qText = "meteo france"}
      && "meteo-france" `elem` ids q0 {Cat.qText = "météorologique"}
      && null (ids q0 {Cat.qText = "no such thing"})
  let br = ids q0 {Cat.qCountry = Just "br"}
  check "search_by_country_keeps_world_sources" $
    "cemaden" `elem` br
      && "gdacs" `elem` br
      && "nws" `notElem` br
      && ids q0 {Cat.qCountry = Just "PT"} == ["meteoalarm", "gdacs"]
  check "filters" $
    all ((== Cat.NoAuth) . Cat.entryAuth) (Cat.search q0 {Cat.qNoToken = True})
      && ids q0 {Cat.qCountry = Just "BR", Cat.qAlertsOnly = True, Cat.qNoToken = True} == ["cemaden", "defesa-civil-idap", "inmet", "gdacs"]
  check "modules_name_real_entries" $
    (Cat.entryModule =<< Cat.getEntry "cemaden") == Just "cemaden" && isNothing (Cat.getEntry "nope")

------------------------------------------------------------------------

panelJson :: String
panelJson =
  "{\"alertas\":[\n\
  \{\"cod_alerta\":3141,\"datahoracriacao\":\"2026-10-25 11:20:44.159\",\"ult_atualizacao\":\"2026-10-25 11:20:44.159\",\"codibge\":4318002,\"evento\":\"Risco Hidrológico - Alto\",\"nivel\":\"Alto\",\"status\":1,\"uf\":\"RS\",\"municipio\":\"SÃO BORJA\",\"latitude\":-28.746805688578,\"longitude\":-55.796610881728},\n\
  \{\"cod_alerta\":3154,\"datahoracriacao\":\"2026-10-25 17:42:03.819\",\"ult_atualizacao\":\"2026-10-25 17:42:03.819\",\"codibge\":3520301,\"evento\":\"Risco Hidrológico - Moderado\",\"nivel\":\"Moderado\",\"status\":1,\"uf\":\"SP\",\"municipio\":\"IGUAPE\",\"latitude\":-24.549631967648,\"longitude\":-47.47466614198},\n\
  \{\"cod_alerta\":3163,\"datahoracriacao\":\"2026-10-27 05:12:39.979\",\"ult_atualizacao\":\"2026-10-27 05:12:39.979\",\"codibge\":2905602,\"evento\":\"Movimentos de Massa - Moderado\",\"nivel\":\"Moderado\",\"status\":1,\"uf\":\"BA\",\"municipio\":\"CAMACAN\",\"latitude\":-15.445822667921,\"longitude\":-39.518998924364}\n\
  \],\"atualizado\":\"29-10-2026 00:45:01 UTC\"}"

replaceOnce :: String -> String -> String -> Maybe String
replaceOnce from to s = go [] s
  where
    go _ [] = Nothing
    go acc r@(c : cs)
      | from `isPrefixOf` r = Just (reverse acc <> to <> drop (length from) r)
      | otherwise = go (c : acc) cs

cemadenTests :: Check -> IO ()
cemadenTests check = do
  case Cemaden.parse (utf8 panelJson) of
    Right all'@[a, _, c] ->
      check "parses_the_panel" $
        length all' == 3
          && alertId a == "3141-20261025112044"
          && source a == Cemaden.sourceId
          && hazard a == Flood
          && severity a == Severe
          && ufs a == [RS]
          && headline a == "Risco Hidrológico - Alto em SÃO BORJA (RS)."
          && area a == "SÃO BORJA (RS), código IBGE 4318002."
          && sent a == "2026-10-25T11:20:44-03:00"
          && hazard c == Landslide
          && severity c == Moderate
    other -> check ("parses_the_panel: " <> show other) False
  check "no_alert_is_valid" (Cemaden.parse "{\"alertas\":[],\"atualizado\":\"x\"}" == Right [])
  let isJson r = case r of Left (Cemaden.Json _) -> True; _ -> False
  check "a_changed_panel_fails_loudly: not the JSON" (isJson (Cemaden.parse "{}") && isJson (Cemaden.parse "<html>"))
  let cases =
        [ ("status", "\"status\":1", "\"status\":0"),
          ("nivel", "\"nivel\":\"Alto\"", "\"nivel\":\"Extremo\""),
          ("evento", "Risco Hidrológico - Alto", "Seca - Alto"),
          ("evento", "Risco Hidrológico - Alto", "Risco Hidrológico - Moderado"),
          ("uf", "\"uf\":\"RS\"", "\"uf\":\"XX\""),
          ("codibge", "4318002", "4106902"),
          ("ult_atualizacao", "\"ult_atualizacao\":\"2026-10-25 11:20:44.159\"", "\"ult_atualizacao\":\"25/10/2026\"")
        ]
  mapM_
    ( \(f, from, to) -> case replaceOnce from to panelJson of
        Nothing -> check ("sample lacks " <> from) False
        Just j -> check ("a_changed_panel_fails_loudly: " <> f) $ case Cemaden.parse (utf8 j) of
          Left (Cemaden.Unknown 3141 f' _) -> f' == f
          _ -> False
    )
    cases
  check "times" $
    Cemaden.time "2026-10-28 04:15:34.38" == Just "2026-10-28T04:15:34-03:00"
      && Cemaden.time "2026-10-28 04:15:34" == Just "2026-10-28T04:15:34-03:00"
      && all (isNothing . Cemaden.time) ["2026-10-28T04:15:34", "2026-10-28 04:15:34.x", ""]

------------------------------------------------------------------------

mockSent :: String
mockSent = "2026-10-27T12:00:00+00:00"

mockTests :: Check -> IO ()
mockTests check = do
  let all' = Mock.scenarios mockSent
  check "every_mock_alert_is_marked_as_a_test" $
    length all' == 5 && all (\a -> "TESTE: " `isPrefixOf` headline a && source a == Mock.sourceId && sent a == mockSent) all'
  check "ids_are_unique_and_hold_the_time" $
    map alertId (take 1 all') == ["202610271200000000-1"]
      && length (nub (map alertId all')) == length all'
      && map alertId (take 1 (Mock.scenarios "2026-10-27T12:00:01+00:00")) /= map alertId (take 1 all')
  let json = "[{\"severity\": \"Severe\", \"ufs\": [\"RS\"], \"headline\": \"rio acima da cota.\", \"area\": \"Área simulada.\"}, {\"hazard\": \"Flood\", \"severity\": \"Minor\", \"ufs\": [\"PR\", \"SC\"], \"headline\": \"TESTE: já marcado.\", \"area\": \"Área simulada.\"}]"
  check "parses_a_mock_file" $ case Mock.parse (utf8 json) mockSent of
    Right [a, b] -> headline a == "TESTE: rio acima da cota." && hazard a == Flood && headline b == "TESTE: já marcado." && ufs b == [PR, SC]
    _ -> False
  let isJson r = case r of Left (Mock.MockJson _) -> True; _ -> False
  check "a_bad_mock_file_fails_loudly" $
    Mock.parse "[]" mockSent == Left Mock.MockEmpty
      && isJson (Mock.parse "{}" mockSent)
      && isJson (Mock.parse "[{\"severity\": \"Severe\", \"ufs\": [\"RS\"], \"headline\": \"x\", \"area\": \"y\", \"id\": \"1\"}]" mockSent)
      && isJson (Mock.parse "[{\"severity\": \"Severe\", \"ufs\": [\"XX\"], \"headline\": \"x\", \"area\": \"y\"}]" mockSent)
  fixture <- BL.readFile "test/fixtures/alerts.json"
  check "the_example_file_parses" (either (const False) (not . null) (Mock.parse fixture mockSent))

------------------------------------------------------------------------

tokenTests :: Check -> IO ()
tokenTests check = do
  let secret = "eyJhbGciOi.secret"
      Just ped = Cat.getEntry "cemaden-ped"
      Just mf = Cat.getEntry "meteo-france"
      Just cem = Cat.getEntry "cemaden"
      st = runIdentity (tokenState ped (const (Identity (Just secret))))
  check "a_token_is_never_shown" $ case st of
    Set tk -> expose tk == secret && not (any ("secret" `isInfixOf`) [show tk, show st, describe st])
    _ -> False
  check "missing_and_empty_tokens_name_the_variable" $
    and
      [ s == Missing "ALERTS_TOKEN_METEO_FRANCE" && describe s == "token missing: set ALERTS_TOKEN_METEO_FRANCE"
      | v <- [Nothing, Just "", Just "  "],
        let s = runIdentity (tokenState mf (const (Identity v)))
      ]
  check "free_sources_need_no_token" (runIdentity (tokenState cem (const (Identity Nothing))) == NotNeeded)
  seen <- newIORef []
  _ <- tokenState ped (\n -> modifyIORef seen (n :) >> pure Nothing)
  names' <- readIORef seen
  check "the_variable_is_the_one_of_the_entry" (names' == ["ALERTS_TOKEN_CEMADEN_PED"])

------------------------------------------------------------------------

envTests :: Check -> IO ()
envTests check = do
  check "env_file_mode" $
    not (readableByOthers 0o100600)
      && not (readableByOthers 0o100400)
      && all readableByOthers [0o100644, 0o100640, 0o100604]
  check "dotenv: pairs, quotes and comments" $
    parseDotenv "# c\nA=1\nexport B = \"x y\"\nC='z'\nD=v # note\nE=\n" == Right [("A", "1"), ("B", "x y"), ("C", "z"), ("D", "v"), ("E", "")]
  check "dotenv: an invalid line is named by number only" (parseDotenv "A=1\nnot a line\n" == Left 2)
  check "only_discord_webhooks" $
    isDiscordWebhook "https://discord.com/api/webhooks/1/abc"
      && isDiscordWebhook "https://discordapp.com/api/webhooks/1/abc"
      && not (isDiscordWebhook "http://discord.com/api/webhooks/1/abc")
      && not (isDiscordWebhook "https://discord.com.example.org/api/webhooks/1")
      && not (isDiscordWebhook "")
  let p = payload (testAlert "") {severity = Extreme}
      embed = case p of
        Object o | Just (Array es) <- KM.lookup "embeds" o, [Object e] <- foldr (:) [] es -> Just e
        _ -> Nothing
      field i = case KM.lookup "fields" =<< embed of
        Just (Array fs) -> case drop i (foldr (:) [] fs) of
          Object f : _ -> KM.lookup "value" f
          _ -> Nothing
        _ -> Nothing
  check "payload_has_pt_br_fields" $
    field 0 == Just (String "extrema")
      && field 1 == Just (String "PR, SC, RS")
      && (KM.lookup "color" =<< embed) == Just (Number 0xD7263D)
      && maybe False (\e -> isNothing (KM.lookup "url" e) && isNothing (KM.lookup "timestamp" e)) embed
  check "alert JSON round trip" (decode (encode sample) == Just sample)
  let _ = object ["x" .= (1 :: Int)]
  pure ()

------------------------------------------------------------------------

serviceTests :: Check -> IO ()
serviceTests check = do
  h <- newHttp
  check "source_ids_round_trip" $
    all (\s -> parseSource (sourceIdOf s) == Right s) allSources && isLeft (parseSource "idap")
  check "every_agency_source_is_in_the_catalog" $
    and [(Cat.entryModule =<< Cat.getEntry (sourceIdOf s)) == Just (sourceIdOf s) | s <- allSources, s /= Mock]
      && and [isRight (parseSource m) | e <- Cat.catalog, Just m <- [Cat.entryModule e]]
  check "alerts_ufs" $
    ufsOf Nothing == Right v0
      && ufsOf (Just " ") == Right v0
      && ufsOf (Just "all") == Right allUfs
      && fmap length (ufsOf (Just "ALL")) == Right 27
      && ufsOf (Just "ba, sp") == Right [BA, SP]
      && isLeft (ufsOf (Just "PR,XX"))
  check "agency_sources_are_actual" (statusOf "cemaden" == Actual)
  check "mock_and_unknown_sources_are_tests" (statusOf "mock" == Test && statusOf "no-such-source" == Test)
  let mocks = Mock.scenarios mockSent
  do
    db <- openMemory
    r <- deliver h db (harness allUfs) noChannels Mock mocks
    check "every_uf_keeps_the_alerts_outside_v0" (fmap passed r == Right 5)
    close db
  do
    db <- openMemory
    first <- deliver h db v0Harness noChannels Mock mocks
    again <- deliver h db v0Harness noChannels Mock mocks
    -- The SP scenario is outside v0: the harness drops it.
    check "delivers_each_alert_once" (first == Right (Report 5 4 4) && fmap delivered again == Right 0)
    close db
  do
    db <- openMemory
    r <- deliver h db v0Harness noChannels Mock (case mocks of a : rest -> a {source = "defesa-civil-idap"} : rest; [] -> [])
    check "a_source_cannot_speak_for_another" (isLeft r)
    close db
  do
    db <- openMemory
    _ <- deliver h db v0Harness noChannels Mock mocks
    tmp <- getTemporaryDirectory
    let path = tmp </> "alerts-feed-test.xml"
    n <- writeFeed db path (FeedMeta "urn:piabrota-alerts" "alerts") mockSent
    xml <- readFile path
    _ <- pure $! length xml
    removeFile path
    let count s = length (filter (s `isPrefixOf`) (tailsOf xml))
    check "the_feed_publishes_mock_alerts_as_tests" $
      n == Right 4
        && count "<status>Test</status>" == 4
        && not ("<status>Actual</status>" `isInfixOf` xml)
        && "TESTE: alerta simulado de inundação de severidade extrema." `isInfixOf` xml
    close db
  do
    -- the rules of the store: a feed, its items, a rule over the feed and the alerts
    db <- openMemory
    addFeed db (FeedRow "g1" "G1" "https://example.org/rss" True)
    _ <- recordItems db "g1" (either (const []) id (P.parse rss "g1"))
    _ <- deliver h db v0Harness noChannels Mock mocks
    let rule = RuleRow "sul" "Sul" (Union [Feed "g1", Filter (MinSeverity Severe) Alerts])
    ok <- checkRule db rule
    addRule db rule
    bad <- checkRule db (RuleRow "loop" "Loop" (Rule "loop"))
    unknown <- checkRule db (RuleRow "nada" "Nada" (Feed "nada"))
    out <- ruleAtom db "sul"
    check "rules: check, store and Atom" $
      ok == Right ()
        && isLeft bad
        && isLeft unknown
        && either (const False) (\x -> "<id>urn:piabrota-alerts:rule:sul</id>" `isInfixOf` x && "<status>Test</status>" `notElem` [x]) out
        && either (const False) (\x -> length (filter ("<entry>" `isPrefixOf`) (tailsOf x)) == 4) out
    close db
  let _ = (sort [()], mapMaybe Just [()], BLC.empty)
  pure ()
