{-# LANGUAGE OverloadedStrings #-}

-- | The mock source: simulated alerts, to try the harness and the channels with no agency.
--
-- Every mock alert is a test. Its headline starts with @TESTE:@, its source is @mock@, and the
-- service publishes it with CAP status Test. A mock alert is never published as an alert of an
-- agency (hard rule "Official sources only").
module Alerts.Source.Mock
  ( sourceId,
    prefix,
    MockAlert (..),
    MockError (..),
    describeMockError,
    scenarios,
    parse,
    alerts,
  )
where

import Alerts.Core
import Data.Aeson
import Data.Aeson.Key qualified as K
import Data.Aeson.KeyMap qualified as KM
import Data.ByteString.Lazy qualified as BL
import Data.Char (isDigit, isSpace)
import Data.List (isPrefixOf)

sourceId :: String
sourceId = "mock"

-- | The start of every mock headline, so a reader never takes a mock alert for a real one.
prefix :: String
prefix = "TESTE:"

-- | One simulated alert, as a person writes it in a mock file. The mock fills the id, the
-- source and the time.
data MockAlert = MockAlert
  { mHazard :: Hazard,
    mSeverity :: Severity,
    mUfs :: [Uf],
    mHeadline :: String,
    mArea :: String
  }
  deriving (Show, Eq)

instance FromJSON MockAlert where
  parseJSON = withObject "MockAlert" $ \o -> do
    case filter (`notElem` ["hazard", "severity", "ufs", "headline", "area"]) (KM.keys o) of
      [] -> pure ()
      extra -> fail ("unknown field " <> show (map K.toString extra))
    MockAlert
      <$> o .:? "hazard" .!= Flood
      <*> o .: "severity"
      <*> o .: "ufs"
      <*> o .: "headline"
      <*> o .: "area"

data MockError
  = -- | The mock file is not a JSON array of mock alerts.
    MockJson String
  | -- | The mock file has no alert. A source never answers an empty list in silence.
    MockEmpty
  deriving (Show, Eq)

describeMockError :: MockError -> String
describeMockError (MockJson e) = "invalid mock file: " <> e
describeMockError MockEmpty = "the mock file has no alert"

-- | The built-in scenarios: one alert per severity in the v0 states, and one alert in SP that
-- the v0 harness drops.
scenarios :: String -> [Alert]
scenarios =
  alerts
    [ mock Minor [PR] "alerta simulado de inundação de severidade menor." "Área simulada no Paraná. Este alerta não é real.",
      mock Moderate [SC] "alerta simulado de inundação de severidade moderada." "Área simulada em Santa Catarina. Este alerta não é real.",
      mock Severe [RS] "alerta simulado de inundação de severidade severa." "Área simulada no Rio Grande do Sul. Este alerta não é real.",
      mock Extreme v0 "alerta simulado de inundação de severidade extrema." "Área simulada no Paraná, em Santa Catarina e no Rio Grande do Sul. Este alerta não é real.",
      mock Severe [SP] "alerta simulado fora da região do v0." "Área simulada em São Paulo. O harness do v0 descarta este alerta."
    ]
  where
    mock = MockAlert Flood

-- | The alerts of a mock file: a JSON array of 'MockAlert'.
parse :: BL.ByteString -> String -> Either MockError [Alert]
parse body t = case eitherDecode body of
  Left e -> Left (MockJson e)
  Right [] -> Left MockEmpty
  Right ms -> Right (alerts ms t)

-- | Turns mock alerts into alerts sent at a time. Each call is a new simulated event: the ids
-- hold the time, so the service delivers the alerts of a new call again.
alerts :: [MockAlert] -> String -> [Alert]
alerts ms t = zipWith one [1 :: Int ..] ms
  where
    stamp = filter isDigit t
    one n m =
      Alert
        { alertId = stamp <> "-" <> show n,
          source = sourceId,
          hazard = mHazard m,
          severity = mSeverity m,
          ufs = mUfs m,
          headline = mark (mHeadline m),
          area = mArea m,
          sent = t,
          url = Nothing
        }

-- | The headline with 'prefix' at the start, added only when it is missing.
mark :: String -> String
mark h0
  | prefix `isPrefixOf` h = h
  | otherwise = prefix <> " " <> h
  where
    h = reverse (dropWhile isSpace (reverse (dropWhile isSpace h0)))
