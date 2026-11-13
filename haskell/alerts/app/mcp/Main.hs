{-# LANGUAGE OverloadedStrings #-}

-- | MCP server (stdio, JSON-RPC 2.0, one message per line) that exposes alerts to agents.
module Main (main) where

import Alerts.Catalog
import Alerts.Core
import Alerts.Service (v0Harness)
import Control.Monad (unless)
import Data.Aeson
import Data.Aeson.Key qualified
import Data.Aeson.KeyMap qualified as KM
import Data.ByteString.Char8 qualified as BC
import Data.ByteString.Lazy qualified as BL
import Data.List (intercalate)
import Data.Maybe (isJust)
import Data.Text qualified as T
import Data.Text.Encoding qualified as TE
import System.IO

main :: IO ()
main = do
  hSetBuffering stdout LineBuffering
  hSetEncoding stdin utf8
  hSetEncoding stdout utf8
  loop
  where
    loop = do
      eof <- isEOF
      unless eof $ do
        line <- BC.hGetLine stdin
        unless (BC.null line) $ case decodeStrict line of
          Just (Object msg) -> mapM_ (BL.putStr . (<> "\n") . encode) (handle msg)
          _ -> BL.putStr (encode (failure Null (-32700) "parse error") <> "\n")
        loop

-- | The answer to one message; a notification gets none.
handle :: Object -> Maybe Value
handle msg = case (KM.lookup "id" msg, KM.lookup "method" msg) of
  (Nothing, _) -> Nothing
  (Just i, Just (String m)) -> Just $ case m of
    "initialize" ->
      success i $
        object
          [ "protocolVersion" .= maybe (String "2025-06-18") id (param "protocolVersion"),
            "capabilities" .= object ["tools" .= object []],
            "serverInfo" .= object ["name" .= ("alerts" :: String), "version" .= ("0.0.0" :: String)]
          ]
    "ping" -> success i (object [])
    "tools/list" -> success i (object ["tools" .= tools])
    "tools/call" -> case (param "name", param "arguments") of
      (Just (String name), args) -> maybe (failure i (-32602) ("unknown tool " <> T.unpack name)) (success i . textResult) (call name (maybe KM.empty asObject args))
      _ -> failure i (-32602) "tools/call needs a name"
    _ -> failure i (-32601) ("unknown method " <> T.unpack m)
  (Just i, _) -> Just (failure i (-32600) "invalid request")
  where
    param k = case KM.lookup "params" msg of
      Just (Object p) -> KM.lookup k p
      _ -> Nothing
    asObject (Object o) = o
    asObject _ = KM.empty

success :: Value -> Value -> Value
success i r = object ["jsonrpc" .= ("2.0" :: String), "id" .= i, "result" .= r]

failure :: Value -> Int -> String -> Value
failure i code m = object ["jsonrpc" .= ("2.0" :: String), "id" .= i, "error" .= object ["code" .= code, "message" .= m]]

textResult :: String -> Value
textResult t = object ["content" .= [object ["type" .= ("text" :: String), "text" .= t]]]

tools :: [Value]
tools =
  [ tool "v0_ufs" "List the Brazilian states (UF) that alerts v0 serves" [],
    tool
      "search_sources"
      "Search the curated catalog of official alert sources, in Brazil and in the world. Each result gives the endpoint, the format, and the environment variable of the token when the source needs your own token (BYOT). Never returns a token."
      [ ("text", "string", "words that must all appear (agency, hazard, format...), any case or accent"),
        ("country", "string", "an ISO 3166-1 alpha-2 country code, for example BR; world sources always match"),
        ("no_token", "boolean", "only the sources that need no token (BYOT)"),
        ("alerts_only", "boolean", "only the sources that give alerts, not observations")
      ],
    tool "check_alert" "Say whether the v0 harness keeps or drops an alert" [("alert_json", "string", "one alerts Alert, as JSON")]
  ]
  where
    tool :: String -> String -> [(T.Text, String, String)] -> Value
    tool n d ps =
      object
        [ "name" .= n,
          "description" .= d,
          "inputSchema"
            .= object
              [ "type" .= ("object" :: String),
                "properties" .= object [(fromText' k, object ["type" .= t, "description" .= pd]) | (k, t, pd) <- ps],
                "required" .= ["alert_json" :: String | n == "check_alert"]
              ]
        ]
    fromText' = Data.Aeson.Key.fromText

call :: T.Text -> Object -> Maybe String
call name args = case name of
  "v0_ufs" -> Just (intercalate ", " (map ufText v0))
  "search_sources" -> Just (BC.unpack (BL.toStrict (encode (map row (search q)))))
  "check_alert" -> Just $ case str "alert_json" >>= decodeStrict . TE.encodeUtf8 . T.pack of
    Nothing -> "invalid alert: not the JSON of an alert"
    Just a -> if isJust (run v0Harness a) then "kept" else "dropped"
  _ -> Nothing
  where
    str k = case KM.lookup k args of
      Just (String t) -> Just (T.unpack t)
      _ -> Nothing
    bool k = case KM.lookup k args of
      Just (Bool b) -> b
      _ -> False
    q = Query (maybe "" id (str "text")) (str "country") (bool "no_token") (bool "alerts_only")
    row e =
      let (tEnv, tHow) = case entryAuth e of
            NoAuth -> (Nothing, Nothing)
            Token v h -> (Just v, Just h)
       in object
            [ "id" .= entryId e,
              "name" .= entryName e,
              "agency" .= entryAgency e,
              "countries" .= entryCountries e,
              "hazards" .= entryHazards e,
              "kind" .= (case entryKind e of AlertsKind -> "alerts"; Observations -> "observations" :: String),
              "format" .= entryFormat e,
              "url" .= entryUrl e,
              "docs" .= entryDocs e,
              "token_env" .= tEnv,
              "token_how" .= tHow,
              "module" .= entryModule e,
              "checked" .= entryChecked e
            ]
