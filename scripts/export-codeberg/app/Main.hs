module Main (main) where

import Export (run)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = getArgs >>= run >>= either (\e -> hPutStrLn stderr e >> exitFailure) putStrLn
