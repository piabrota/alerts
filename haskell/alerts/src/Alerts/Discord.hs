{-# LANGUAGE OverloadedStrings #-}

-- | The Discord channel: the webhook body of an alert and its delivery.
module Alerts.Discord
  ( isDiscordWebhook,
    payload,
    post,
  )
where

import Alerts.Core
import Alerts.Http
import Data.Aeson
import Data.Aeson.KeyMap qualified as KM
import Data.List (intercalate, isPrefixOf)
import Data.Text qualified as T

-- | A Discord webhook URL, the only kind of URL that 'post' accepts.
isDiscordWebhook :: String -> Bool
isDiscordWebhook u =
  any (`isPrefixOf` u) ["https://discord.com/api/webhooks/", "https://discordapp.com/api/webhooks/"]

-- | The Discord webhook body for one alert: one embed, pt-BR labels.
payload :: Alert -> Value
payload a = object ["embeds" .= [Object (KM.fromList (base <> extra))]]
  where
    base =
      [ ("title", toJSON (headline a)),
        ("description", toJSON (area a)),
        ("color", toJSON (severityColor (severity a))),
        ( "fields",
          toJSON
            [ field "Severidade" (severityLabelPtBr (severity a)),
              field "UF" (intercalate ", " (map ufText (ufs a))),
              field "Fonte" (source a)
            ]
        )
      ]
    -- Discord rejects null or empty url and timestamp, so add them only when present.
    extra =
      [("url", toJSON u) | Just u <- [url a]]
        <> [("timestamp", toJSON (sent a)) | not (null (sent a))]
    field :: String -> String -> Value
    field n v = object ["name" .= n, "value" .= v, "inline" .= True]

-- | Posts one alert to a Discord webhook and returns the id of the message that Discord stored.
-- An error never contains the webhook URL: the URL holds the webhook token.
post :: Http -> String -> Alert -> IO (Either String String)
post h webhook a
  | not (isDiscordWebhook webhook) = pure (Left "the webhook URL is not a Discord webhook URL")
  | otherwise = do
      -- wait=true makes Discord answer with the stored message, so a 2xx proves delivery.
      let sep = if '?' `elem` webhook then "&" else "?"
      r <- postJson h (webhook <> sep <> "wait=true") (encode (payload a))
      pure $ case r of
        Left e -> Left ("Discord: " <> e)
        Right body -> case decode body of
          Just (Object o) | Just (String i) <- KM.lookup "id" o, i /= "" -> Right (T.unpack i)
          _ -> Left "Discord answered without a message id"
