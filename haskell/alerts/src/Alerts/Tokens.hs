-- | BYOT (bring your own token): the tokens of the catalog sources that need one.
--
-- alerts never ships a token. Each person registers with the agency and sets the token in the
-- variable that the catalog names (@ALERTS_TOKEN_<ID>@), in the environment or in @.env@. A
-- token is a secret like a webhook URL: it never reaches a message, a log or an error.
module Alerts.Tokens
  ( Token,
    expose,
    TokenState (..),
    describe,
    tokenState,
  )
where

import Alerts.Catalog (Auth (..), Entry (..))
import Data.Char (isSpace)

-- | A token. 'Show' never shows it; only 'expose' gives the value, to put it in a request.
newtype Token = Secret String
  deriving (Eq)

instance Show Token where
  show _ = "Token(<hidden>)"

expose :: Token -> String
expose (Secret t) = t

-- | The token state of one catalog entry.
data TokenState
  = NotNeeded
  | Set Token
  | Missing String
  deriving (Show, Eq)

-- | The state as one short line for people, never with the token.
describe :: TokenState -> String
describe NotNeeded = "no token"
describe (Set _) = "token set"
describe (Missing v) = "token missing: set " <> v

-- | The token of an entry, read by a lookup (the environment, in the service).
tokenState :: (Monad m) => Entry -> (String -> m (Maybe String)) -> m TokenState
tokenState e lookupVar = case entryAuth e of
  NoAuth -> pure NotNeeded
  Token v _ -> do
    got <- lookupVar v
    pure $ case trim <$> got of
      Just t | not (null t) -> Set (Secret t)
      _ -> Missing v
  where
    trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace
