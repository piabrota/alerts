{-# LANGUAGE OverloadedStrings #-}

-- | The CEMADEN source: the municipal alerts of the CEMADEN alert panel.
--
-- The panel publishes its alerts in an undocumented JSON ('url'). It needs no token. The official
-- PED API needs a token (BYOT, see "Alerts.Catalog"), but it gives observations, not alerts. An
-- undocumented endpoint can change without notice, so every field this module does not
-- understand is an error, never a dropped alert.
--
-- The module does no I/O: the service reads the panel, and this module turns its text into
-- alerts.
module Alerts.Source.Cemaden
  ( sourceId,
    panelUrl,
    panelPage,
    CemadenError (..),
    describeCemadenError,
    parse,
    time,
  )
where

import Alerts.Core
import Data.Aeson
import Data.ByteString.Lazy qualified as BL
import Data.Char (isDigit)
import Data.List (isPrefixOf)

-- | The source id of every CEMADEN alert.
sourceId :: String
sourceId = "cemaden"

-- | The JSON of the active municipal alerts.
panelUrl :: String
panelUrl = "https://painelalertas.cemaden.gov.br/wsAlertas2"

-- | The panel page, for people. Every alert links to it.
panelPage :: String
panelPage = "https://painelalertas.cemaden.gov.br/"

-- | The CEMADEN times have no offset. They are taken as Brasília time (UTC-3) ⚠: nobody checked
-- it with CEMADEN.
offset :: String
offset = "-03:00"

data Panel = Panel [Alerta] String

data Alerta = Alerta
  { codAlerta :: Integer,
    ultAtualizacao :: String,
    codIbge :: Integer,
    evento :: String,
    nivel :: String,
    status :: Integer,
    ufField :: String,
    municipio :: String
  }

instance FromJSON Panel where
  parseJSON = withObject "Panel" $ \o -> Panel <$> o .: "alertas" <*> o .: "atualizado"

instance FromJSON Alerta where
  parseJSON = withObject "Alerta" $ \o ->
    Alerta
      <$> (nonNegative =<< o .: "cod_alerta")
      <*> o .: "ult_atualizacao"
      <*> (nonNegative =<< o .: "codibge")
      <*> o .: "evento"
      <*> o .: "nivel"
      <*> o .: "status"
      <*> o .: "uf"
      <*> o .: "municipio"
    where
      nonNegative n = if n < 0 then fail "negative number" else pure n

data CemadenError
  = -- | The answer is not the JSON of the panel.
    Json String
  | -- | A field has a value that this module does not know. The panel changed: a human must look.
    Unknown Integer String String
  deriving (Show, Eq)

describeCemadenError :: CemadenError -> String
describeCemadenError (Json e) = "the CEMADEN panel answer is not the known JSON: " <> e
describeCemadenError (Unknown a f v) = "CEMADEN alert " <> show a <> ": unknown " <> f <> " " <> show v

-- | The alerts of one answer of 'panelUrl'. An empty list is valid: no municipality is under alert.
parse :: BL.ByteString -> Either CemadenError [Alert]
parse body = case eitherDecode body of
  Left e -> Left (Json e)
  Right (Panel as _) -> traverse alert as

alert :: Alerta -> Either CemadenError Alert
alert a = do
  let unknown f v = Left (Unknown (codAlerta a) f v)
  -- Every alert seen on 2026-10-28 had status 1. Another value may mean an ended alert ⚠.
  if status a /= 1 then unknown "status" (show (status a)) else Right ()
  uf <- maybe (unknown "uf" (ufField a)) Right (parseUf (ufField a))
  if fromIbge (show (codIbge a)) /= Just uf then unknown "codibge" (show (codIbge a)) else Right ()
  sev <- maybe (unknown "nivel" (nivel a)) Right (severityOf (nivel a))
  (kind, level) <- maybe (unknown "evento" (evento a)) Right (splitOnce " - " (evento a))
  if level /= nivel a then unknown "evento" (evento a) else Right ()
  hz <- case kind of
    "Risco Hidrológico" -> Right Flood
    "Movimentos de Massa" -> Right Landslide
    _ -> unknown "evento" (evento a)
  t <- maybe (unknown "ult_atualizacao" (ultAtualizacao a)) Right (time (ultAtualizacao a))
  -- The id holds the update time: when CEMADEN changes an alert, the service delivers it again.
  let stamp = filter isDigit (take 19 t)
      u = ufText uf
  pure
    Alert
      { alertId = show (codAlerta a) <> "-" <> stamp,
        source = sourceId,
        hazard = hz,
        severity = sev,
        ufs = [uf],
        headline = evento a <> " em " <> municipio a <> " (" <> u <> ").",
        area = municipio a <> " (" <> u <> "), código IBGE " <> show (codIbge a) <> ".",
        sent = t,
        url = Just panelPage
      }

splitOnce :: String -> String -> Maybe (String, String)
splitOnce sep = go []
  where
    go _ [] = Nothing
    go acc s@(c : cs)
      | sep `isPrefixOf` s = Just (reverse acc, drop (length sep) s)
      | otherwise = go (c : acc) cs

-- | The CEMADEN levels as CAP severities. CEMADEN has three levels: moderate, high and very high
-- (@Muito Alto@ was not seen in the panel ⚠).
severityOf :: String -> Maybe Severity
severityOf n = case n of
  "Moderado" -> Just Moderate
  "Alto" -> Just Severe
  "Muito Alto" -> Just Extreme
  _ -> Nothing

-- | @2026-10-25 11:20:44.159@ as @2026-10-25T11:20:44-03:00@.
time :: String -> Maybe String
time t
  | ok = Just (take 10 t <> "T" <> take 8 (drop 11 t) <> offset)
  | otherwise = Nothing
  where
    at i = t !! i
    ok =
      length t >= 19
        && all (isDigit . at) [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
        && all (\(i, c) -> at i == c) [(4, '-'), (7, '-'), (10, ' '), (13, ':'), (16, ':')]
        && (length t == 19 || (at 19 == '.' && all isDigit (drop 20 t)))
