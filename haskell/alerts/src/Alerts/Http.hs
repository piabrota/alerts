-- | HTTP with the honest User-Agent. Every request of alerts goes through this module, so every
-- request sends 'userAgent'.
module Alerts.Http
  ( Http,
    newHttp,
    get,
    postJson,
  )
where

import Alerts.Core (userAgent)
import Control.Exception (try)
import Data.ByteString.Char8 qualified as BC
import Data.ByteString.Lazy qualified as BL
import Network.HTTP.Client
import Network.HTTP.Client.TLS (newTlsManager)
import Network.HTTP.Types (hContentType, hUserAgent, statusCode)

newtype Http = Http Manager

newHttp :: IO Http
newHttp = Http <$> newTlsManager

-- | The error never holds the URL: a webhook URL holds its token.
send :: Http -> Request -> IO (Either String BL.ByteString)
send (Http m) req = do
  r <- try (httpLbs req {requestHeaders = (hUserAgent, BC.pack userAgent) : requestHeaders req} m)
  pure $ case r of
    Left e -> Left (describe e)
    Right resp
      | code >= 200 && code < 300 -> Right (responseBody resp)
      | otherwise -> Left ("HTTP status " <> show code)
      where
        code = statusCode (responseStatus resp)
  where
    describe :: HttpException -> String
    describe (HttpExceptionRequest _ c) = case c of
      ResponseTimeout -> "the request timed out"
      ConnectionTimeout -> "the connection timed out"
      ConnectionFailure _ -> "the connection failed"
      _ -> "the request failed"
    describe (InvalidUrlException _ _) = "invalid URL"

-- | The body of a @GET@.
get :: Http -> String -> IO (Either String BL.ByteString)
get h u = case parseRequest u of
  Nothing -> pure (Left "invalid URL")
  Just req -> send h req

-- | The answer body of a JSON @POST@.
postJson :: Http -> String -> BL.ByteString -> IO (Either String BL.ByteString)
postJson h u body = case parseRequest u of
  Nothing -> pure (Left "invalid URL")
  Just req ->
    send h req {method = BC.pack "POST", requestBody = RequestBodyLBS body, requestHeaders = [(hContentType, BC.pack "application/json")]}
