-- | The environment of the service: @.env@ and the variables it reads.
module Alerts.Env
  ( loadDotenv,
    parseDotenv,
    env,
    readableByOthers,
  )
where

import Control.Monad (forM_, unless, when)
import Data.Bits ((.&.))
import Data.Char (isAlphaNum, isSpace)
import Data.List (isPrefixOf)
import Numeric (showOct)
import System.Directory (doesFileExist, getCurrentDirectory)
import System.Environment (lookupEnv, setEnv)
import System.FilePath (takeDirectory, (</>))
import System.IO (hPutStrLn, stderr)
import System.Posix.Files (fileMode, getFileStatus)

-- | An environment variable, or 'Nothing' when it is unset or empty. @.env.example@ lists every
-- variable with an empty value, so an empty variable means "not set".
env :: String -> IO (Maybe String)
env name = do
  v <- lookupEnv name
  pure $ case v of
    Just s | not (null s) -> Just s
    _ -> Nothing

-- | The pairs of a @.env@ text, or the number of the first invalid line. The error never holds
-- the line: it may hold a secret.
parseDotenv :: String -> Either Int [(String, String)]
parseDotenv = traverse line . filter (not . skip . snd) . zip [1 ..] . lines
  where
    skip l = let t = dropWhile isSpace l in null t || "#" `isPrefixOf` t
    line (n, l0) =
      let l = dropWhile isSpace (maybe l0 id (stripExport (dropWhile isSpace l0)))
          (k, rest) = span (\c -> isAlphaNum c || c == '_' || c == '.') l
       in case dropWhile isSpace rest of
            '=' : v | not (null k) -> maybe (Left n) (\x -> Right (k, x)) (value (dropWhile isSpace v))
            _ -> Left n
    stripExport s = if "export " `isPrefixOf` s then Just (drop 7 s) else Nothing
    value v = case v of
      '"' : r -> quoted '"' r
      '\'' : r -> quoted '\'' r
      _ -> Just (trimEnd (cutComment v))
    -- An unquoted value ends at " #": a '#' with no space before it is part of the value.
    cutComment s = case s of
      [] -> []
      c : '#' : _ | isSpace c -> [c]
      c : r -> c : cutComment r
    quoted q r = case break (== q) r of
      (x, _ : after) | all isSpace (takeWhile (/= '#') after) -> Just x
      _ -> Nothing
    trimEnd = reverse . dropWhile isSpace . reverse

-- | True when the group or other users have any permission on the file.
readableByOthers :: Integer -> Bool
readableByOthers mode = mode .&. 0o077 /= 0

-- | Loads @.env@ (from the current directory or a parent) into the environment. A variable that
-- is already set wins, and a missing @.env@ is fine. An invalid line fails without its text: the
-- line may hold a webhook token. A @.env@ that other users can read prints a warning to stderr.
loadDotenv :: IO (Either String ())
loadDotenv = do
  found <- findUp =<< getCurrentDirectory
  case found of
    Nothing -> pure (Right ())
    Just path -> do
      st <- getFileStatus path
      let mode = toInteger (fileMode st)
      when (readableByOthers mode) $
        hPutStrLn stderr $
          "warning: other users can read " <> path <> " (mode " <> showOct (mode .&. 0o777) "" <> "); run: chmod 600 " <> path
      text <- readFile path
      case parseDotenv text of
        Left n -> pure (Left (".env: invalid line " <> show n <> " (the line is not printed: it may hold a secret)"))
        Right pairs -> do
          forM_ pairs $ \(k, v) -> do
            old <- lookupEnv k
            unless (maybe False (const True) old) (setEnv k v)
          pure (Right ())
  where
    findUp dir = do
      let p = dir </> ".env"
      ok <- doesFileExist p
      if ok
        then pure (Just p)
        else
          let up = takeDirectory dir
           in if up == dir then pure Nothing else findUp up
