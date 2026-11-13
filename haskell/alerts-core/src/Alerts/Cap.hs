-- | The CAP 1.2 @<identifier>@ of an alert. The reference is @identChars@ and
-- @identifier@ in @spec/Alerts/Core.agda@; @identifier-unique@ in
-- @spec/Alerts/Properties.agda@ states when it is unique.
module Alerts.Cap
  ( sender,
    identChars,
    capSafe,
    identifier,
  )
where

import Alerts.Core (Alert (..))
import Data.Char (isSpace)

sender :: String
sender = "piabrota-alerts"

-- | The sender, the source id and the alert id, joined by @-@.
identChars :: String -> String -> String
identChars src i = sender <> "-" <> src <> "-" <> i

-- | CAP forbids whitespace and @, < > & " '@ in an identifier: they become @_@.
capSafe :: Char -> Char
capSafe c
  | isSpace c || c `elem` (",<>&\"'" :: String) = '_'
  | otherwise = c

identifier :: Alert -> String
identifier a = map capSafe (identChars (source a) (alertId a))
