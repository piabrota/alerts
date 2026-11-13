-- | Feed times. RSS 2.0 uses RFC 822 (@Tue, 27 Oct 2026 03:00:00 GMT@), Atom uses RFC 3339
-- (@2026-10-27T03:00:00Z@). An item keeps one form only: UTC as
-- @YYYY-MM-DDThh:mm:ss+00:00@, the form that CAP 1.2 and "Alerts.Feed" accept. In that form the
-- text order is the time order.
module Alerts.Time
  ( normalize,
    rfc3339Utc,
    daysFromCivil,
    epochOf,
  )
where

import Data.Char (isDigit, isSpace, toLower)
import Data.List (stripPrefix)
import Text.Printf (printf)

-- | A feed time in the UTC form, or 'Nothing' when the text is not RFC 3339 or RFC 822.
normalize :: String -> Maybe String
normalize raw = rfc3339Utc <$> epochOf raw

-- | Seconds since 1970-01-01T00:00:00Z of an RFC 3339 or RFC 822 time.
epochOf :: String -> Maybe Integer
epochOf raw = case parseRfc3339 s of
  Just t -> Just t
  Nothing -> parseRfc822 s
  where
    s = trim raw

trim :: String -> String
trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace

-- | Seconds since 1970-01-01T00:00:00Z as @YYYY-MM-DDThh:mm:ss+00:00@.
rfc3339Utc :: Integer -> String
rfc3339Utc secs =
  printf "%04d-%02d-%02dT%02d:%02d:%02d+00:00" y m d h mi s
  where
    (days, rem') = secs `divMod` 86400
    -- Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    z = days + 719468
    era = z `div` 146097
    doe = z - era * 146097
    yoe = (doe - doe `quot` 1460 + doe `quot` 36524 - doe `quot` 146096) `quot` 365
    doy = doe - (365 * yoe + yoe `quot` 4 - yoe `quot` 100)
    mp = (5 * doy + 2) `quot` 153
    d = doy - (153 * mp + 2) `quot` 5 + 1
    m = if mp < 10 then mp + 3 else mp - 9
    y = yoe + era * 400 + (if m <= 2 then 1 else 0) :: Integer
    (h, mi, s) = (rem' `quot` 3600, rem' `rem` 3600 `quot` 60, rem' `rem` 60)

-- | Days since 1970-01-01 of a civil date (the inverse of the algorithm above).
daysFromCivil :: Integer -> Integer -> Integer -> Integer
daysFromCivil y0 m d = era * 146097 + doe - 719468
  where
    y = if m <= 2 then y0 - 1 else y0
    era = y `div` 400
    yoe = y - era * 400
    mp = if m > 2 then m - 3 else m + 9
    doy = (153 * mp + 2) `quot` 5 + d - 1
    doe = yoe * 365 + yoe `quot` 4 - yoe `quot` 100 + doy

epoch :: (Integer, Integer, Integer) -> (Integer, Integer, Integer) -> Integer -> Maybe Integer
epoch (y, m, d) (h, mi, s) offsetMin
  | ok = Just (daysFromCivil y m d * 86400 + h * 3600 + mi * 60 + s - offsetMin * 60)
  | otherwise = Nothing
  where
    within lo hi v = lo <= v && v <= hi
    ok = within 1 12 m && within 1 31 d && within 0 23 h && within 0 59 mi && within 0 60 s

num :: String -> Maybe Integer
num s
  | not (null s) && all isDigit s = Just (read s)
  | otherwise = Nothing

splitOn :: Char -> String -> [String]
splitOn c s = case break (== c) s of
  (a, []) -> [a]
  (a, _ : rest) -> a : splitOn c rest

-- | @YYYY-MM-DDThh:mm:ss[.fff](Z|±hh:mm)@.
parseRfc3339 :: String -> Maybe Integer
parseRfc3339 s = do
  let (date, rest0) = break (`elem` ("Tt " :: String)) s
  rest <- case rest0 of
    _ : r -> Just r
    [] -> Nothing
  ymd <- case splitOn '-' date of
    [a, b, c] -> (,,) <$> num a <*> num b <*> num c
    _ -> Nothing
  if length rest < 8 then Nothing else Just ()
  let (clock, zone0) = splitAt 8 rest
  hms <- case splitOn ':' clock of
    [a, b, c] -> (,,) <$> num a <*> num b <*> num c
    _ -> Nothing
  let zone = maybe zone0 (dropWhile isDigit) (stripPrefix "." zone0)
  offset <- case zone of
    "Z" -> Just 0
    "z" -> Just 0
    sign : hm -> do
      (h, m) <- case splitOn ':' hm of
        [a, b] -> Just (a, b)
        _ -> Nothing
      off <- (\a b -> a * 60 + b) <$> num h <*> num m
      case sign of
        '+' -> Just off
        '-' -> Just (negate off)
        _ -> Nothing
    [] -> Nothing
  epoch ymd hms offset

-- | @[Day, ]DD Mon YYYY hh:mm[:ss] zone@, with a two-digit year allowed by RFC 822.
parseRfc822 :: String -> Maybe Integer
parseRfc822 s0 = do
  let s = case break (== ',') s0 of
        (_, _ : rest) -> rest
        _ -> s0
  (dw, mw, yw, cw, rest) <- case words s of
    (a : b : c : d : r) -> Just (a, b, c, d, r)
    _ -> Nothing
  d <- num dw
  m <- if length mw < 3 then Nothing else lookup (map toLower (take 3 mw)) months
  y0 <- num yw
  let y
        | y0 < 50 = 2000 + y0
        | y0 < 100 = 1900 + y0
        | otherwise = y0
  hms <- case splitOn ':' cw of
    [a, b] -> (,,) <$> num a <*> num b <*> Just 0
    a : b : c : _ -> (,,) <$> num a <*> num b <*> num c
    _ -> Nothing
  offset <- case take 1 rest of
    [] -> Just 0
    [z] -> zoneOffset z
    _ -> Nothing
  epoch (y, m, d) hms offset
  where
    months = zip ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"] [1 ..]
    zoneOffset z = case z of
      _ | z `elem` ["GMT", "UT", "UTC", "Z"] -> Just 0
      "EDT" -> Just (-4 * 60)
      _ | z `elem` ["EST", "CDT"] -> Just (-5 * 60)
      _ | z `elem` ["CST", "MDT"] -> Just (-6 * 60)
      _ | z `elem` ["MST", "PDT"] -> Just (-7 * 60)
      "PST" -> Just (-8 * 60)
      sign : hm
        | length hm == 4 -> do
            off <- (\a b -> a * 60 + b) <$> num (take 2 hm) <*> num (drop 2 hm)
            case sign of
              '+' -> Just off
              '-' -> Just (negate off)
              _ -> Nothing
      _ -> Nothing
