{-# LANGUAGE OverloadedStrings #-}

-- | RSS 2.0, RSS 1.0 (RDF) and Atom (RFC 4287) documents to items. The parser reads only the
-- fields an 'Item' holds and ignores every other element and namespace.
module Alerts.Rss.Parse
  ( ParseError (..),
    parse,
  )
where

import Alerts.Rss.Item
import Alerts.Time (normalize)
import Control.Exception (displayException)
import Data.Char (isSpace)
import Data.List (find)
import Data.Map.Strict qualified as Map
import Data.Text qualified as T
import Data.Text.Lazy qualified as TL
import Text.XML

data ParseError
  = -- | The document is not well-formed XML.
    Xml String
  | -- | The root element is not @rss@, @RDF@ or @feed@.
    NotAFeed String
  deriving (Show, Eq)

data Draft = Draft
  { dId, dTitle, dLink, dSummary, dContent, dUpdated, dPublished :: String
  }

emptyDraft :: Draft
emptyDraft = Draft "" "" "" "" "" "" ""

trim :: String -> String
trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace

finish :: String -> Draft -> Maybe Item
finish feed d = do
  let title = trim (dTitle d)
      link = let l = trim (dLink d) in if null l then Nothing else Just l
  key <- find (not . null) (maybe [] pure link <> [trim (dId d), title])
  let summary = if null (trim (dSummary d)) then dContent d else dSummary d
  pure
    Item
      { itemFeed = feed,
        itemKey = key,
        itemTitle = title,
        itemLink = link,
        itemSummary = trim summary,
        itemUpdated = maybe (normalize (dPublished d)) Just (normalize (dUpdated d)),
        itemAlert = Nothing
      }

-- | Appends text to the field an element name feeds, if any.
field :: String -> String -> Draft -> Maybe Draft
field name t d = case name of
  "title" -> Just d {dTitle = dTitle d <> t}
  "guid" -> Just d {dId = dId d <> t}
  "id" -> Just d {dId = dId d <> t}
  "link" -> Just d {dLink = dLink d <> t}
  "description" -> Just d {dSummary = dSummary d <> t}
  "summary" -> Just d {dSummary = dSummary d <> t}
  "content" -> Just d {dContent = dContent d <> t}
  "encoded" -> Just d {dContent = dContent d <> t}
  _ | name `elem` ["updated", "pubDate", "date", "modified"] -> Just d {dUpdated = dUpdated d <> t}
  _ | name `elem` ["published", "issued"] -> Just d {dPublished = dPublished d <> t}
  _ -> Nothing

local :: Element -> String
local = T.unpack . nameLocalName . elementName

-- | The @href@ of an Atom @<link>@, when its @rel@ is absent or @alternate@.
atomHref :: Element -> Maybe String
atomHref e = case Map.lookup "rel" attrs of
  Just rel | rel /= "alternate" -> Nothing
  _ -> T.unpack <$> Map.lookup "href" attrs
  where
    attrs = Map.mapKeys nameLocalName (elementAttributes e)

-- | Every text below a node, in document order.
textOf :: Node -> String
textOf (NodeContent t) = T.unpack t
textOf (NodeElement e) = concatMap textOf (elementNodes e)
textOf _ = ""

item :: Element -> Draft
item e = foldl child emptyDraft [c | NodeElement c <- elementNodes e]
  where
    child d c =
      let name = local c
          d' = if name == "link" then maybe d (\h -> d {dLink = h}) (atomHref c) else d
       in maybe d' id (field name (concatMap textOf (elementNodes c)) d')

-- | The item and entry elements below a node, without the ones inside another item.
items :: Element -> [Element]
items e = concatMap go [c | NodeElement c <- elementNodes e]
  where
    go c
      | local c `elem` ["item", "entry"] = [c]
      | otherwise = items c

-- | Every item of the document, in document order, tagged with @feed@. An entry with no link,
-- no id and no title has no identity, so it is left out.
parse :: String -> String -> Either ParseError [Item]
parse xml feed = case parseText def {psDecodeEntities = decodeHtmlEntities} (TL.pack xml) of
  Left err
    | all isSpace xml -> Left (NotAFeed "")
    | otherwise -> Left (Xml (displayException err))
  Right doc ->
    let root = documentRoot doc
     in if local root `elem` ["rss", "RDF", "feed"]
          then Right [i | e <- items root, Just i <- [finish feed (item e)]]
          else Left (NotAFeed (local root))
