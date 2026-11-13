-- | The Atom + CAP 1.2 channel: an Atom feed (RFC 4287) whose entries each hold one CAP 1.2
-- alert (OASIS), the same shape as the national IDAP feed.
module Alerts.Feed
  ( capNs,
    Status (..),
    FeedMeta (..),
    FeedError (..),
    checkTime,
    escape,
    cap,
    Entry (..),
    atom,
    atomEntries,
  )
where

import Alerts.Cap (identifier, sender)
import Alerts.Core
import Data.Char (isDigit)

capNs :: String
capNs = "urn:oasis:names:tc:emergency:cap:1.2"

-- | CAP @<status>@. A test alert must never be published as 'Actual'.
data Status = Actual | Test
  deriving (Show, Eq)

data FeedMeta = FeedMeta
  { -- | A permanent URI for the feed, for example @urn:piabrota-alerts@.
    feedId :: String,
    feedTitle :: String
  }
  deriving (Show, Eq)

-- | @sent@ is not @YYYY-MM-DDThh:mm:ss±hh:mm@, the only form CAP 1.2 accepts.
newtype FeedError = InvalidTime String
  deriving (Show, Eq)

checkTime :: String -> Either FeedError String
checkTime t
  | ok = Right t
  | otherwise = Left (InvalidTime t)
  where
    at i = t !! i
    ok =
      length t == 25
        && all (isDigit . at) [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18, 20, 21, 23, 24]
        && all (\(i, c) -> at i == c) [(4, '-'), (7, '-'), (10, 'T'), (13, ':'), (16, ':'), (22, ':')]
        && at 19 `elem` ("+-" :: String)

-- | The five XML escapes.
escape :: String -> String
escape = concatMap esc
  where
    esc '&' = "&amp;"
    esc '<' = "&lt;"
    esc '>' = "&gt;"
    esc '"' = "&quot;"
    esc '\'' = "&apos;"
    esc c = [c]

capSeverity :: Severity -> String
capSeverity = show

-- | One CAP 1.2 @<alert>@ element, without an XML declaration.
cap :: Alert -> Status -> Either FeedError String
cap a status = do
  t <- checkTime (sent a)
  pure $
    concat
      [ "<alert xmlns=\"",
        capNs,
        "\"><identifier>",
        escape (identifier a),
        "</identifier><sender>",
        sender,
        "</sender><sent>",
        t,
        "</sent><status>",
        show status,
        "</status><msgType>Alert</msgType>",
        "<source>",
        escape (source a),
        "</source><scope>Public</scope><info><language>pt-BR</language>",
        "<category>",
        capCategory (hazard a),
        "</category><event>",
        hazardLabelPtBr (hazard a),
        "</event><urgency>Unknown</urgency><severity>",
        capSeverity (severity a),
        "</severity><certainty>Unknown</certainty><headline>",
        escape (headline a),
        "</headline>",
        maybe "" (\u -> "<web>" <> escape u <> "</web>") (url a),
        "<area><areaDesc>",
        escape (area a),
        "</areaDesc>",
        concatMap (\u -> "<geocode><valueName>UF</valueName><value>" <> ufText u <> "</value></geocode>") (ufs a),
        "</area></info></alert>"
      ]

-- | One Atom @<entry>@. An entry that carries an alert holds it as CAP 1.2 in @<content>@, so a
-- composed feed delivers the alert with the same text as every other channel.
data Entry = Entry
  { entryId :: String,
    entryTitle :: String,
    entryLink :: Maybe String,
    -- | In the same form as 'sent'.
    entryUpdated :: String,
    entrySummary :: String,
    entryAlert :: Maybe (Alert, Status)
  }

-- | The whole Atom feed. @updated@ is the feed time, in the same form as 'sent'.
atom :: FeedMeta -> [Alert] -> Status -> String -> Either FeedError String
atom meta as status = atomEntries meta (map entry as)
  where
    entry a =
      Entry
        { entryId = feedId meta <> ":" <> identifier a,
          entryTitle = headline a,
          entryLink = Nothing,
          entryUpdated = sent a,
          entrySummary = area a,
          entryAlert = Just (a, status)
        }

-- | An Atom feed of any entries, with or without an alert.
atomEntries :: FeedMeta -> [Entry] -> String -> Either FeedError String
atomEntries meta entries updated = do
  t <- checkTime updated
  es <- traverse one entries
  pure $
    concat
      [ "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<feed xmlns=\"http://www.w3.org/2005/Atom\"><id>",
        escape (feedId meta),
        "</id><title>",
        escape (feedTitle meta),
        "</title><updated>",
        t,
        "</updated><author><name>",
        sender,
        "</name></author>"
      ]
      <> concat es
      <> "</feed>\n"
  where
    one e = do
      u <- checkTime (entryUpdated e)
      c <- traverse (\(a, s) -> cap a s) (entryAlert e)
      pure $
        concat
          [ "<entry><id>",
            escape (entryId e),
            "</id><title>",
            escape (entryTitle e),
            "</title>",
            maybe "" (\l -> "<link href=\"" <> escape l <> "\"/>") (entryLink e),
            "<updated>",
            u,
            "</updated><summary>",
            escape (entrySummary e),
            "</summary>",
            maybe "" (\x -> "<content type=\"application/cap+xml\">" <> x <> "</content>") c,
            "</entry>"
          ]
