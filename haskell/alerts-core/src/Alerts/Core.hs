{-# LANGUAGE OverloadedStrings #-}

-- | Domain types and the FP harness of alerts.
--
-- A harness is a composition of pure stages. Each stage takes an 'Alert' and
-- returns @Just alert@ to pass it on, possibly changed, or @Nothing@ to drop
-- it. Specializing a harness means composing one more stage, never overriding
-- a method.
--
-- The reference of this module is @spec/Alerts/Core.agda@: every definition
-- here has a twin of the same name there, and @test/Spec.hs@ replays the
-- theorems of @spec/Alerts/Properties.agda@ on this code.
module Alerts.Core
  ( -- * Identity
    userAgent,

    -- * UF
    Uf (..),
    allUfs,
    v0,
    ufText,
    parseUf,
    ibgeCode,
    ufOfPrefix,
    fromIbge,
    trim,

    -- * Severity and hazard
    Severity (..),
    allSeverities,
    rank,
    severityLabelPtBr,
    severityColor,
    severityCssColor,
    Hazard (..),
    hazardLabelPtBr,
    capCategory,

    -- * Alerts
    Alert (..),
    parseUfExact,

    -- * The FP harness
    Stage,
    Harness,
    emptyHarness,
    andThen,
    stages,
    run,
    keepIf,
    onlyUfs,
    minSeverity,
    deliver,
  )
where

import Control.Monad ((>=>))
import Data.Aeson (FromJSON (..), ToJSON (..), object, withObject, withText, (.:), (.:?), (.=))
import Data.Char (isDigit, isSpace, ord, toUpper)
import Data.List (dropWhileEnd, find)
import Data.Maybe (mapMaybe)
import Data.Text qualified as T
import Data.Version (showVersion)
import Numeric (showHex)
import Paths_alerts_core (version)

-- | The User-Agent of every alerts HTTP request: an honest name and a way to
-- reach us.
userAgent :: String
userAgent = "piabrota-alerts/" <> showVersion version <> " (+https://github.com/piabrota/alerts)"

------------------------------------------------------------------------
-- UF

-- | A Brazilian federative unit (state or Distrito Federal).
data Uf
  = AC
  | AL
  | AP
  | AM
  | BA
  | CE
  | DF
  | ES
  | GO
  | MA
  | MT
  | MS
  | MG
  | PA
  | PB
  | PR
  | PE
  | PI
  | RJ
  | RN
  | RS
  | RO
  | RR
  | SC
  | SP
  | SE
  | TO
  deriving (Show, Read, Eq, Ord, Enum, Bounded)

allUfs :: [Uf]
allUfs = [minBound .. maxBound]

-- | The states v0 serves: the South region.
v0 :: [Uf]
v0 = [PR, SC, RS]

ufText :: Uf -> String
ufText = show

-- | Parses @"RS"@, @" rs "@: trims and ignores case.
parseUf :: String -> Maybe Uf
parseUf s = find ((== map toUpper (trim s)) . ufText) allUfs

-- | The JSON form of a UF is its uppercase code, exactly: @"RS"@.
parseUfExact :: String -> Maybe Uf
parseUfExact s = find ((== s) . ufText) allUfs

instance ToJSON Uf where
  toJSON = toJSON . ufText

instance FromJSON Uf where
  parseJSON = withText "Uf" $ \t ->
    maybe (fail ("unknown UF: " <> show t)) pure (parseUfExact (T.unpack t))

-- | The two-digit IBGE code of a UF.
ibgeCode :: Uf -> Int
ibgeCode u = case u of
  RO -> 11
  AC -> 12
  AM -> 13
  RR -> 14
  PA -> 15
  AP -> 16
  TO -> 17
  MA -> 21
  PI -> 22
  CE -> 23
  RN -> 24
  PB -> 25
  PE -> 26
  AL -> 27
  SE -> 28
  BA -> 29
  MG -> 31
  ES -> 32
  RJ -> 33
  SP -> 35
  PR -> 41
  SC -> 42
  RS -> 43
  MS -> 50
  MT -> 51
  GO -> 52
  DF -> 53

-- | The UF of a two-digit prefix: the inverse of 'ibgeCode', read off it.
ufOfPrefix :: Int -> Maybe Uf
ufOfPrefix p = find ((== p) . ibgeCode) allUfs

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

-- | The UF of an IBGE code: a 2-digit UF code or a 7-digit municipality code,
-- whose first two digits are the UF code.
fromIbge :: String -> Maybe Uf
fromIbge raw
  | all isAsciiDigit cs,
    length cs `elem` [2, 7],
    (a : b : _) <- cs =
      ufOfPrefix (10 * digit a + digit b)
  | otherwise = Nothing
  where
    cs = trim raw
    digit c = ord c - ord '0'
    isAsciiDigit c = isDigit c

------------------------------------------------------------------------
-- Severity and hazard

-- | CAP severity, ordered from lowest to highest.
data Severity = Minor | Moderate | Severe | Extreme
  deriving (Show, Read, Eq, Ord, Enum, Bounded)

allSeverities :: [Severity]
allSeverities = [minBound .. maxBound]

rank :: Severity -> Int
rank = fromEnum

-- | The pt-BR label every channel shows, so all channels say the same thing.
severityLabelPtBr :: Severity -> String
severityLabelPtBr s = case s of
  Minor -> "menor"
  Moderate -> "moderada"
  Severe -> "severa"
  Extreme -> "extrema"

-- | The color of a severity as @0xRRGGBB@, one palette for every channel and
-- the map: blue, then the yellow, orange and red of the INMET warning levels.
severityColor :: Severity -> Int
severityColor s = case s of
  Minor -> 0x1682D4
  Moderate -> 0xF5C518
  Severe -> 0xF28C28
  Extreme -> 0xD7263D

-- | The color as a CSS hex string, for example @#d7263d@.
severityCssColor :: Severity -> String
severityCssColor s = '#' : pad (showHex (severityColor s) "")
  where
    pad h = replicate (6 - length h) '0' <> h

data Hazard
  = Flood
  | -- | Landslides, mudslides and other mass movements (CEMADEN: "Movimentos de Massa").
    Landslide
  deriving (Show, Read, Eq, Ord, Enum, Bounded)

instance ToJSON Severity where
  toJSON = toJSON . show

instance FromJSON Severity where
  parseJSON = withText "Severity" $ \t ->
    maybe (fail ("unknown severity: " <> show t)) pure (find ((== T.unpack t) . show) allSeverities)

instance ToJSON Hazard where
  toJSON = toJSON . show

instance FromJSON Hazard where
  parseJSON = withText "Hazard" $ \t ->
    maybe (fail ("unknown hazard: " <> show t)) pure (find ((== T.unpack t) . show) [minBound .. maxBound :: Hazard])

hazardLabelPtBr :: Hazard -> String
hazardLabelPtBr Flood = "Inundação"
hazardLabelPtBr Landslide = "Movimento de massa"

-- | The CAP 1.2 @<category>@ of a hazard.
capCategory :: Hazard -> String
capCategory Flood = "Met"
capCategory Landslide = "Geo"

------------------------------------------------------------------------
-- Alerts

data Alert = Alert
  { -- | Stable id given by the source, used for deduplication.
    alertId :: String,
    -- | Source module id, for example @inmet@.
    source :: String,
    hazard :: Hazard,
    severity :: Severity,
    ufs :: [Uf],
    -- | Controlled pt-BR text, one statement per sentence.
    headline :: String,
    area :: String,
    -- | ISO 8601 time the source sent the alert.
    sent :: String,
    url :: Maybe String
  }
  deriving (Show, Eq)

-- | The JSON form of the previous implementation: @id@, @source@, @hazard@, @severity@, @ufs@,
-- @headline@, @area@, @sent@, @url@.
instance ToJSON Alert where
  toJSON a =
    object
      [ "id" .= alertId a,
        "source" .= source a,
        "hazard" .= hazard a,
        "severity" .= severity a,
        "ufs" .= ufs a,
        "headline" .= headline a,
        "area" .= area a,
        "sent" .= sent a,
        "url" .= url a
      ]

instance FromJSON Alert where
  parseJSON = withObject "Alert" $ \o ->
    Alert
      <$> o .: "id"
      <*> o .: "source"
      <*> o .: "hazard"
      <*> o .: "severity"
      <*> o .: "ufs"
      <*> o .: "headline"
      <*> o .: "area"
      <*> o .: "sent"
      <*> o .:? "url"

------------------------------------------------------------------------
-- The FP harness

-- | Pass the alert on, possibly changed, or drop it.
type Stage = Alert -> Maybe Alert

-- | An ordered composition of stages. The empty harness passes every alert.
newtype Harness = Harness [Stage]

emptyHarness :: Harness
emptyHarness = Harness []

-- | Composes one more stage after the others.
andThen :: Harness -> Stage -> Harness
andThen (Harness ss) s = Harness (ss <> [s])

stages :: [Stage] -> Harness
stages = Harness

-- | Kleisli composition of the stages, in order.
run :: Harness -> Alert -> Maybe Alert
run (Harness ss) = foldr (>=>) Just ss

-- | A stage that keeps or drops, never changes.
keepIf :: (Alert -> Bool) -> Stage
keepIf p a = if p a then Just a else Nothing

-- | Keeps alerts that touch at least one of @keep@.
onlyUfs :: [Uf] -> Stage
onlyUfs keep = keepIf (any (`elem` keep) . ufs)

-- | Keeps alerts at @m@ severity or above.
minSeverity :: Severity -> Stage
minSeverity m = keepIf ((m <=) . severity)

-- | A channel delivers what the harness passes, from a batch of alerts.
deliver :: Harness -> [Alert] -> [Alert]
deliver h = mapMaybe (run h)
