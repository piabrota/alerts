{-# LANGUAGE OverloadedStrings #-}

-- | The rule algebra. An 'Expr' is a value: it has a JSON form, the store keeps it, and 'eval'
-- reads it. Every operation works on item /sets/: the identity of an item is its key, and each
-- result is in one canonical form (one item per key, newest first, then by key). So the usual
-- laws hold as equalities:
--
-- * @union@ and @intersect@ are associative, commutative and idempotent;
-- * @diff(a, a)@ is empty, and @diff(a, b)@ never holds a key of @b@;
-- * @filter(p, filter(q, a)) = filter(all[p, q], a)@;
-- * @filter(p, union(a, b)) = union(filter(p, a), filter(p, b))@ when no key is in two feeds.
--
-- When two feeds hold the same key, the item of the feed whose slug sorts first is kept, so the
-- choice does not depend on the order of the operands.
module Alerts.Rss.Expr
  ( Expr (..),
    Pred (..),
    TextField (..),
    MapOp (..),
    Env (..),
    emptyEnv,
    EvalError (..),
    describeEvalError,
    Refs (..),
    refs,
    eval,
    canonical,
    foldText,
    stripHtml,
    atomOfItems,
    entryIdOf,
  )
where

import Alerts.Core (Severity, Uf, minSeverity, onlyUfs)
import Alerts.Feed
import Alerts.Rss.Item
import Alerts.Time (normalize)
import Control.Monad (unless)
import Data.Aeson
import Data.Aeson.Key qualified as K
import Data.Aeson.KeyMap qualified as KM
import Data.Aeson.Types (Parser)
import Data.Char (isSpace, toLower)
import Data.List (elemIndex, isInfixOf, isPrefixOf, sortBy)
import Data.Map.Strict qualified as Map
import Data.Maybe (isJust)
import Data.Ord (Down (..), comparing)
import Data.Set qualified as Set

-- | A rule: the items of a composed feed.
data Expr
  = -- | The items of one feed, by slug.
    Feed String
  | -- | The official alerts, each as one item that carries its CAP alert.
    Alerts
  | -- | The result of another rule, by slug. A rule cannot reach itself.
    Rule String
  | -- | The items of any operand. @union []@ is empty.
    Union [Expr]
  | -- | The items whose key is in every operand. @intersect []@ is empty.
    Intersect [Expr]
  | -- | The items of the left operand whose key is not in the right one.
    Diff Expr Expr
  | Filter Pred Expr
  | -- | Each item changed. The key and the time do not change, and an item that carries an
    -- alert is never changed: a channel never rewrites an alert.
    Map MapOp Expr
  | -- | One item per title (case and spaces ignored): the newest one.
    Dedupe Expr
  | -- | The @n@ newest items.
    Take Int Expr
  deriving (Show, Eq)

-- | A test on one item.
data Pred
  = -- | The text is in the field, case ignored.
    Contains TextField String
  | -- | The item time is at or after this time (RFC 3339 or RFC 822). An item without a time fails.
    Since String
  | -- | The item comes from one of these feeds (@alerts@ names the official alerts).
    From [String]
  | -- | The item carries an alert at this severity or above (the 'minSeverity' stage).
    MinSeverity Severity
  | -- | The item carries an alert that touches one of these UFs (the 'onlyUfs' stage).
    Ufs [Uf]
  | Not Pred
  | -- | Every test holds. @all []@ always holds.
    All [Pred]
  | -- | At least one test holds. @any []@ never holds.
    Any [Pred]
  deriving (Show, Eq)

data TextField = TitleField | SummaryField | AnyField
  deriving (Show, Eq)

-- | A change to one item.
data MapOp
  = -- | Puts the text before the title, for example @[G1] @.
    PrefixTitle String
  | -- | Removes the HTML tags of the summary.
    StripHtml
  | -- | Keeps the first characters of the summary, and adds @…@ when it cut.
    Truncate Int
  deriving (Show, Eq)

-- | What a rule reads: the stored items of each feed, the official alerts, and the other rules.
data Env = Env
  { envFeeds :: Map.Map String [Item],
    envAlerts :: [Item],
    envRules :: Map.Map String Expr
  }
  deriving (Show, Eq)

emptyEnv :: Env
emptyEnv = Env Map.empty [] Map.empty

data EvalError
  = UnknownFeed String
  | UnknownRule String
  | -- | The rules in the order they reach each other; the last one is the first one again.
    Cycle [String]
  | -- | A @since@ time that is not RFC 3339 or RFC 822.
    BadTime String
  deriving (Show, Eq)

describeEvalError :: EvalError -> String
describeEvalError e = case e of
  UnknownFeed s -> "o feed " <> s <> " não existe"
  UnknownRule s -> "a regra " <> s <> " não existe"
  Cycle path -> "ciclo de regras: " <> joinWith " → " path
  BadTime t -> "tempo inválido: " <> t

joinWith :: String -> [String] -> String
joinWith _ [] = ""
joinWith sep (x : xs) = x <> concatMap (sep <>) xs

-- | What an expression names directly, without following its rules.
data Refs = Refs
  { refFeeds :: Set.Set String,
    refRules :: Set.Set String,
    refAlerts :: Bool
  }
  deriving (Show, Eq)

refs :: Expr -> Refs
refs = go (Refs Set.empty Set.empty False)
  where
    go r e = case e of
      Feed f -> r {refFeeds = Set.insert f (refFeeds r)}
      Rule n -> r {refRules = Set.insert n (refRules r)}
      Alerts -> r {refAlerts = True}
      Union es -> foldl' go r es
      Intersect es -> foldl' go r es
      Diff a b -> go (go r a) b
      Filter _ a -> go r a
      Map _ a -> go r a
      Dedupe a -> go r a
      Take _ a -> go r a

-- | The items of an expression in canonical form. The function is pure: the same rule and the
-- same environment give the same items.
eval :: Expr -> Env -> Either EvalError [Item]
eval expr env = go [] expr
  where
    go stack e = case e of
      Feed f -> maybe (Left (UnknownFeed f)) (Right . canonical) (Map.lookup f (envFeeds env))
      Alerts -> Right (canonical (envAlerts env))
      Rule n -> case elemIndex n stack' of
        Just i -> Left (Cycle (drop i stack' <> [n]))
        Nothing -> do
          r <- maybe (Left (UnknownRule n)) Right (Map.lookup n (envRules env))
          go (n : stack) r
        where
          stack' = reverse stack
      Union es -> canonical . concat <$> traverse (go stack) es
      Intersect es -> do
        sets <- traverse (go stack) es
        let keys s = Set.fromList (map itemKey s)
            common = case map keys sets of
              [] -> Set.empty
              k : ks -> foldl' Set.intersection k ks
        pure (canonical (filter ((`Set.member` common) . itemKey) (concat sets)))
      Diff l r -> do
        dropKeys <- Set.fromList . map itemKey <$> go stack r
        filter (not . (`Set.member` dropKeys) . itemKey) <$> go stack l
      Filter p a -> do
        checkPred p
        filter (holds p) <$> go stack a
      Map op a -> map (apply op) <$> go stack a
      Dedupe a -> dedupe <$> go stack a
      Take n a -> take n <$> go stack a

    dedupe = reverse . snd . foldl' step (Set.empty, [])
      where
        step (seen, acc) i
          | Set.member t seen = (seen, acc)
          | otherwise = (Set.insert t seen, i : acc)
          where
            t = foldText (itemTitle i)

-- | One item per key (the winner of 'wins'), newest first, then by key. Items without a time
-- come last.
canonical :: [Item] -> [Item]
canonical is = sortBy order (Map.elems (foldl' add Map.empty is))
  where
    add m i = Map.insertWith pick (itemKey i) i m
    pick new old = if wins new old then new else old
    order = comparing (Down . itemUpdated) <> comparing itemKey

-- | On a key clash, the item of the feed whose slug sorts first wins; the other fields break a
-- tie, so the winner never depends on the order of the operands.
wins :: Item -> Item -> Bool
wins a b = tup a < tup b
  where
    tup i = (itemFeed i, itemTitle i, itemSummary i, itemLink i, itemUpdated i)

checkPred :: Pred -> Either EvalError ()
checkPred p = case p of
  Since t -> unless (isJust (normalize t)) (Left (BadTime t))
  Not q -> checkPred q
  All qs -> mapM_ checkPred qs
  Any qs -> mapM_ checkPred qs
  _ -> Right ()

-- | Lowercase, with each run of spaces as one space.
foldText :: String -> String
foldText = map toLower . unwords . words

holds :: Pred -> Item -> Bool
holds p i = case p of
  Contains f text ->
    let t = foldText text
        has s = t `isInfixOf` foldText s
     in case f of
          TitleField -> has (itemTitle i)
          SummaryField -> has (itemSummary i)
          AnyField -> has (itemTitle i) || has (itemSummary i)
  Since t -> case (normalize t, itemUpdated i) of
    (Just t', Just u) -> u >= t'
    _ -> False
  From fs -> itemFeed i `elem` fs
  MinSeverity s -> maybe False (isJust . minSeverity s) (itemAlert i)
  Ufs us -> maybe False (isJust . onlyUfs us) (itemAlert i)
  Not q -> not (holds q i)
  All qs -> all (`holds` i) qs
  Any qs -> any (`holds` i) qs

apply :: MapOp -> Item -> Item
apply op i
  | isJust (itemAlert i) = i
  | otherwise = case op of
      PrefixTitle t -> i {itemTitle = t <> itemTitle i}
      StripHtml -> i {itemSummary = stripHtml (itemSummary i)}
      Truncate n
        | length (itemSummary i) > n -> i {itemSummary = take n (itemSummary i) <> "…"}
        | otherwise -> i

-- | The text outside @<...>@, with the spaces folded. It is for a summary, not a sanitizer: the
-- Atom output escapes the text anyway.
stripHtml :: String -> String
stripHtml = unwords . words . go False
  where
    go _ [] = []
    go False ('<' : cs) = go True cs
    go True ('<' : cs) = go True cs
    go True ('>' : cs) = ' ' : go False cs
    go True (_ : cs) = go True cs
    go False (c : cs) = c : go False cs

-- | The Atom feed of the items, through the Atom + CAP 1.2 channel. An item that carries an
-- alert holds it as CAP 1.2 with status 'Actual'. @updated@ is the feed time, and the time of an
-- item that has none.
atomOfItems :: FeedMeta -> [Item] -> String -> Either FeedError String
atomOfItems meta is updated = atomEntries meta (map entry is) updated
  where
    entry i =
      Entry
        { entryId = entryIdOf meta i,
          entryTitle = itemTitle i,
          entryLink = itemLink i,
          entryUpdated = maybe updated id (itemUpdated i),
          entrySummary = itemSummary i,
          entryAlert = (\a -> (a, Actual)) <$> itemAlert i
        }

-- | The key when it is already a URI; else the key under the feed id, as the alert channel does
-- for an alert.
entryIdOf :: FeedMeta -> Item -> String
entryIdOf meta i
  | any (`isPrefixOf` key) ["http://", "https://", "urn:", "tag:"] = key
  | itemFeed i == alertsFeed = feedId meta <> ":" <> key
  | otherwise = feedId meta <> ":" <> itemFeed i <> ":" <> key
  where
    key = map (\c -> if isSpace c then '_' else c) (itemKey i)

------------------------------------------------------------------------
-- JSON: the form of the previous implementation, with unknown fields refused.

-- | Fails when the object has a key outside the list.
only :: [Key] -> Object -> Parser ()
only ks o = case filter (`notElem` ks) (KM.keys o) of
  [] -> pure ()
  extra -> fail ("unknown field " <> show (map K.toString extra))

tagged :: Key -> (String -> Object -> Parser a) -> Value -> Parser a
tagged tag f = withObject "tagged" $ \o -> do
  t <- o .: tag
  f t o

instance FromJSON Expr where
  parseJSON = tagged "op" $ \t o -> case t of
    "feed" -> only ["op", "feed"] o >> Feed <$> o .: "feed"
    "alerts" -> only ["op"] o >> pure Alerts
    "rule" -> only ["op", "rule"] o >> Rule <$> o .: "rule"
    "union" -> only ["op", "of"] o >> Union <$> o .: "of"
    "intersect" -> only ["op", "of"] o >> Intersect <$> o .: "of"
    "diff" -> only ["op", "left", "right"] o >> Diff <$> o .: "left" <*> o .: "right"
    "filter" -> only ["op", "when", "of"] o >> Filter <$> o .: "when" <*> o .: "of"
    "map" -> only ["op", "with", "of"] o >> Map <$> o .: "with" <*> o .: "of"
    "dedupe" -> only ["op", "of"] o >> Dedupe <$> o .: "of"
    "take" ->
      only ["op", "n", "of"] o >> do
        n <- o .: "n"
        if n < 0 then fail "take: n must not be negative" else Take n <$> o .: "of"
    _ -> fail ("unknown op " <> show t)

instance ToJSON Expr where
  toJSON e = case e of
    Feed f -> object ["op" .= ("feed" :: String), "feed" .= f]
    Alerts -> object ["op" .= ("alerts" :: String)]
    Rule r -> object ["op" .= ("rule" :: String), "rule" .= r]
    Union es -> object ["op" .= ("union" :: String), "of" .= es]
    Intersect es -> object ["op" .= ("intersect" :: String), "of" .= es]
    Diff l r -> object ["op" .= ("diff" :: String), "left" .= l, "right" .= r]
    Filter p a -> object ["op" .= ("filter" :: String), "when" .= p, "of" .= a]
    Map m a -> object ["op" .= ("map" :: String), "with" .= m, "of" .= a]
    Dedupe a -> object ["op" .= ("dedupe" :: String), "of" .= a]
    Take n a -> object ["op" .= ("take" :: String), "n" .= n, "of" .= a]

instance FromJSON Pred where
  parseJSON = tagged "is" $ \t o -> case t of
    "contains" -> only ["is", "field", "text"] o >> Contains <$> o .: "field" <*> o .: "text"
    "since" -> only ["is", "time"] o >> Since <$> o .: "time"
    "from" -> only ["is", "feeds"] o >> From <$> o .: "feeds"
    "min_severity" -> only ["is", "severity"] o >> MinSeverity <$> o .: "severity"
    "ufs" -> only ["is", "ufs"] o >> Ufs <$> o .: "ufs"
    "not" -> only ["is", "p"] o >> Not <$> o .: "p"
    "all" -> only ["is", "of"] o >> All <$> o .: "of"
    "any" -> only ["is", "of"] o >> Any <$> o .: "of"
    _ -> fail ("unknown predicate " <> show t)

instance ToJSON Pred where
  toJSON p = case p of
    Contains f t -> object ["is" .= ("contains" :: String), "field" .= f, "text" .= t]
    Since t -> object ["is" .= ("since" :: String), "time" .= t]
    From fs -> object ["is" .= ("from" :: String), "feeds" .= fs]
    MinSeverity s -> object ["is" .= ("min_severity" :: String), "severity" .= s]
    Ufs us -> object ["is" .= ("ufs" :: String), "ufs" .= us]
    Not q -> object ["is" .= ("not" :: String), "p" .= q]
    All qs -> object ["is" .= ("all" :: String), "of" .= qs]
    Any qs -> object ["is" .= ("any" :: String), "of" .= qs]

instance FromJSON TextField where
  parseJSON = withText "TextField" $ \t -> case t of
    "title" -> pure TitleField
    "summary" -> pure SummaryField
    "any" -> pure AnyField
    _ -> fail ("unknown field " <> show t)

instance ToJSON TextField where
  toJSON f = String $ case f of
    TitleField -> "title"
    SummaryField -> "summary"
    AnyField -> "any"

instance FromJSON MapOp where
  parseJSON = tagged "do" $ \t o -> case t of
    "prefix_title" -> only ["do", "text"] o >> PrefixTitle <$> o .: "text"
    "strip_html" -> only ["do"] o >> pure StripHtml
    "truncate" ->
      only ["do", "chars"] o >> do
        n <- o .: "chars"
        if n < 0 then fail "truncate: chars must not be negative" else pure (Truncate n)
    _ -> fail ("unknown map " <> show t)

instance ToJSON MapOp where
  toJSON m = case m of
    PrefixTitle t -> object ["do" .= ("prefix_title" :: String), "text" .= t]
    StripHtml -> object ["do" .= ("strip_html" :: String)]
    Truncate n -> object ["do" .= ("truncate" :: String), "chars" .= n]
