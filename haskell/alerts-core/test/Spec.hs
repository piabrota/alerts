-- | The theorems of spec/Alerts/Properties.agda, replayed on the Haskell code.
--
-- Agda proves each fact for the specification; this suite checks that the
-- Haskell code agrees with it, exhaustively where the domain is finite
-- (UF, severity, two-digit prefixes) and over every short text where it is
-- not (the CAP identifier). Each check names its Agda theorem.
module Main (main) where

import Alerts.Cap
import Alerts.Core
import Control.Monad (unless)
import Data.IORef
import Data.Maybe (isJust, isNothing)
import System.Exit (exitFailure)

alert :: [Uf] -> Severity -> Alert
alert us s =
  Alert
    { alertId = "t-1",
      source = "test",
      hazard = Flood,
      severity = s,
      ufs = us,
      headline = "Teste.",
      area = "Porto Alegre",
      sent = "2026-10-22T12:00:00-03:00",
      url = Nothing
    }

-- Every alert shape the finite checks walk: no UF, one UF, two UFs.
alerts :: [Alert]
alerts =
  [alert us s | s <- allSeverities, us <- [] : [[u] | u <- allUfs] <> [[u, v] | u <- [AC, PR, SP], v <- [RS, TO]]]

main :: IO ()
main = do
  failures <- newIORef (0 :: Int)
  let check name ok = unless ok $ do
        putStrLn ("FAIL " <> name)
        modifyIORef failures (+ 1)

  -- the unit tests of the previous alerts-core
  check "uf_round_trip" (all (\u -> parseUf (ufText u) == Just u) allUfs)
  check "uf_round_trip: trim and case" (parseUf " rs " == Just RS && isNothing (parseUf "XX"))
  check "uf_round_trip: 27 UFs" (length allUfs == 27)
  check "uf_from_ibge" $
    fromIbge "4314902" == Just RS
      && fromIbge "4106902" == Just PR
      && fromIbge "42" == Just SC
      && fromIbge "3304557" == Just RJ
      && isNothing (fromIbge "34")
      && isNothing (fromIbge "431490")
      && isNothing (fromIbge "43a4902")
  check "one_palette" (severityCssColor Extreme == "#d7263d" && severityCssColor Minor == "#1682d4")
  check "harness_composes_stages" $
    let h = emptyHarness `andThen` onlyUfs v0 `andThen` minSeverity Moderate
     in isJust (run h (alert [RS] Severe))
          && isNothing (run h (alert [SP] Severe))
          && isNothing (run h (alert [SC] Minor))
          && isJust (run emptyHarness (alert [] Minor))

  -- P3, the harness
  check "run-empty" (all (\a -> run emptyHarness a == Just a) alerts)
  let hs =
        [ [],
          [onlyUfs v0],
          [minSeverity Severe],
          [onlyUfs [SP], minSeverity Moderate],
          [\a -> Just a {headline = "x"}]
        ]
  check "run-++" $
    and [run (stages (h <> k)) a == (run (stages h) a >>= run (stages k)) | h <- hs, k <- hs, a <- alerts]
  let keeps = [touches_ ks | ks <- [[], v0, [SP], allUfs]] <> [(m <=) . severity | m <- allSeverities]
  check "keepIf-commute" $
    and [run (stages [keepIf p, keepIf q]) a == run (stages [keepIf q, keepIf p]) a | p <- keeps, q <- keeps, a <- alerts]
  check "stages-commute" $
    and
      [ run (stages [onlyUfs ks, minSeverity m]) a == run (stages [minSeverity m, onlyUfs ks]) a
      | ks <- [[], v0, [SP], allUfs],
        m <- allSeverities,
        a <- alerts
      ]
  check "keepIf-same" (and [maybe True (== a) (keepIf p a) | p <- keeps, a <- alerts])
  check "parity" $
    let channel = stages [onlyUfs v0, minSeverity Minor]
     in deliver channel alerts == deliver channel alerts

  -- P2, severity
  check "rank-injective" (and [(rank a == rank b) == (a == b) | a <- allSeverities, b <- allSeverities])
  check "minSeverity-pass/drop" $
    and [(minSeverity m a == Just a) == (m <= severity a) | m <- allSeverities, a <- alerts]
  check "upward-closed" $
    and
      [ isJust (minSeverity m (alert [RS] s2))
      | m <- allSeverities,
        s <- allSeverities,
        s2 <- allSeverities,
        isJust (minSeverity m (alert [RS] s)),
        s <= s2
      ]
  check "minor-passes-all" (all (\a -> minSeverity Minor a == Just a) alerts)

  -- P1, routing
  let prefixes = [0 .. 99] :: [Int]
  check "ufOfPrefix-sound" (and [maybe True ((== p) . ibgeCode) (ufOfPrefix p) | p <- prefixes])
  check "ufOfPrefix-complete" (all (\u -> ufOfPrefix (ibgeCode u) == Just u) allUfs)
  check "routing-injective" $
    and [p == q | p <- prefixes, q <- prefixes, Just u <- [ufOfPrefix p], ufOfPrefix q == Just u]
  check "ibgeCode-injective" (and [(ibgeCode u == ibgeCode v) == (u == v) | u <- allUfs, v <- allUfs])
  check "south-is-v0" (map ufOfPrefix [41, 42, 43] == map Just v0)
  check "seven-is-two" $
    and [fromIbge (two p <> k) == fromIbge (two p) | p <- prefixes, k <- ["00000", "12345", "99999"]]
  check "seven-is-two: matches ufOfPrefix" (and [fromIbge (two p) == ufOfPrefix p | p <- prefixes])
  check "non-digit-unrouted" (all (isNothing . fromIbge) ["43a4902", "4x", "43 4902", "-4"])
  check "wrong-length-unrouted" (all (isNothing . fromIbge) ["", "4", "431490", "431490200"])
  check "fromIbge-trims" (fromIbge " 43 " == Just RS)

  -- P6, the CAP identifier
  check "identifier-clash" (identChars "cemaden" "ped-1" == identChars "cemaden-ped" "1")
  let texts = concatMap (\n -> mapM (const "a-1") [1 .. n]) [0 .. 3 :: Int]
      srcs = filter (not . dashDigit) texts
      ids = filter startsDigit texts
      pairs = [(s, i) | s <- srcs, i <- ids]
  check "identifier-unique" $
    and [(s1, i1) == (s2, i2) | (s1, i1) <- pairs, (s2, i2) <- pairs, identChars s1 i1 == identChars s2 i2]
  check "identifier: CAP-safe" $
    identifier (alert [] Minor) {source = "a b", alertId = "x,y<z>"} == "piabrota-alerts-a_b-x_y_z_"

  n <- readIORef failures
  if n == 0 then putStrLn "spec: every theorem holds on the Haskell code" else exitFailure
  where
    touches_ ks = any (`elem` ks) . ufs
    two p = let s = show p in replicate (2 - length s) '0' <> s

-- The hypotheses of identifier-unique, as in spec/Alerts/Properties.agda.
dashDigit :: String -> Bool
dashDigit (c : d : cs) = (c == '-' && d `elem` ['0' .. '9']) || dashDigit (d : cs)
dashDigit _ = False

startsDigit :: String -> Bool
startsDigit (c : _) = c `elem` ['0' .. '9']
startsDigit [] = False
