-- | One entry of a feed, the value that the rule algebra ("Alerts.Rss.Expr") works on.
module Alerts.Rss.Item
  ( alertsFeed,
    Item (..),
    fromAlert,
  )
where

import Alerts.Cap (identifier)
import Alerts.Core
import Alerts.Time (normalize)

-- | The feed slug of the items that come from the official alerts. No feed can use it.
alertsFeed :: String
alertsFeed = "alerts"

data Item = Item
  { -- | The slug of the feed the item came from.
    itemFeed :: String,
    -- | The identity of the item in every set operation: its link, else its guid or Atom id,
    -- else its title.
    itemKey :: String,
    itemTitle :: String,
    itemLink :: Maybe String,
    itemSummary :: String,
    -- | The UTC form of 'normalize', or 'Nothing' when the feed gave no valid time.
    itemUpdated :: Maybe String,
    -- | The official alert this item carries, unchanged. Only the alerts leaf sets it.
    itemAlert :: Maybe Alert
  }
  deriving (Show, Eq)

-- | An official alert as an item. The Atom output embeds the alert as CAP 1.2.
fromAlert :: Alert -> Item
fromAlert a =
  Item
    { itemFeed = alertsFeed,
      itemKey = identifier a,
      itemTitle = headline a,
      itemLink = url a,
      itemSummary = area a,
      itemUpdated = normalize (sent a),
      itemAlert = Just a
    }
