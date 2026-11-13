-- | Build the Codeberg copy of alerts with no trace of AI tools.
--
-- Usage: @just export-codeberg <out-dir> [<git-ref>]@, or @export-codeberg --check <dir>@ to run
-- only the trace check on a built tree (the site that @just deploy-site@ publishes).
--
-- Writes the tree of @<git-ref>@ (default @HEAD@) into @<out-dir>@, without the agent files,
-- with GROUND_TRUTH.md as CONTRIBUTING.md and each skill in verbs/ as
-- @docs/guides/<verb>.md@. The tool fails when a trace remains. It never writes git history:
-- commit the result with a plain message and no co-author trailer.
module Export
  ( run,
    traceIn,
    cutRecipe,
    cutBullet,
    plain,
    replaceWord,
  )
where

import Control.Exception (evaluate)
import Control.Monad (forM, forM_, when)
import Data.Char (isAlphaNum, isAsciiLower, toLower)
import Data.List (isInfixOf, isPrefixOf, isSuffixOf, sort)
import System.Directory
import System.Exit (ExitCode (..))
import System.FilePath (makeRelative, takeFileName, (</>))
import System.Process (CreateProcess (..), StdStream (..), createPipe, createProcess, proc, readProcessWithExitCode, waitForProcess)

usage :: String
usage = "usage: export-codeberg <out-dir> [<git-ref>]\n       export-codeberg --check <dir>   (only the trace check, for a built site)"

dropList :: [FilePath]
dropList = ["verbs", "scripts", "opencode.json", ".mcp.json"]

run :: [String] -> IO (Either String String)
run args = case args of
  ["--check", dir] -> fmap (const ("export: no AI trace in " <> dir)) <$> check dir
  [out] -> export out "HEAD"
  [out, r] -> export out r
  _ -> pure (Left usage)

export :: FilePath -> String -> IO (Either String String)
export out0 gitRef = do
  out <- makeAbsolute out0
  exists <- doesDirectoryExist out
  busy <- if exists then any (/= ".git") <$> listDirectory out else pure False
  if busy
    then pure (Left ("export: " <> out <> " must be empty (a .git directory is allowed)"))
    else do
      createDirectoryIfMissing True out
      ok <- extract gitRef out
      if not ok
        then pure (Left ("export: cannot extract " <> gitRef))
        else do
          writeGuides out
          forM_ dropList $ \n -> do
            let p = out </> n
            isDir <- doesDirectoryExist p
            isFile <- doesFileExist p
            when isDir (removeDirectoryRecursive p)
            when isFile (removeFile p)
          -- The recipes lose the export and the lines that run it: scripts/ is not in the copy.
          edit (out </> "justfile") $ \t ->
            unlines' (filter (not . ("export-codeberg" `isInfixOf`)) (lines' (cutRecipe (cutRecipe t "# Zip each skill in verbs/") "# Build the Codeberg copy")))
          edit (out </> "cabal.project") (unlines' . filter (not . ("scripts/export-codeberg" `isInfixOf`)) . lines')
          edit (out </> ".gitignore") (replaceAll "\n# Agent instructions live in GROUND_TRUTH.md\nAGENTS.md\n" "\n")
          renameFile (out </> "GROUND_TRUTH.md") (out </> "CONTRIBUTING.md")
          edit (out </> "CONTRIBUTING.md") $ \t ->
            replaceAll "(MCP server, stdio JSON-RPC, configured in `.mcp.json` and `opencode.json`)" "(MCP server, stdio JSON-RPC: `just mcp`)" $
              cutBullet (header t) "- **Codeberg mirror without AI trace:**"
          files <- textFiles out
          forM_ files $ \p -> do
            t <- readText p
            forM_ t $ \s -> writeFile p (plain s)
          r <- check out
          pure (fmap (const ("export: clean tree of " <> gitRef <> " in " <> out)) r)
  where
    header t =
      let h = "# GROUND_TRUTH.md\n\n"
       in if h `isPrefixOf` t
            then case breakOn "\n\n" (drop (length h) t) of
              Just (_, rest) -> "# Contributing\n\nRules for everyone who works on `alerts`.\n\n" <> rest
              Nothing -> t
            else t

-- | @git archive <ref> | tar -x -C <out>@.
extract :: String -> FilePath -> IO Bool
extract gitRef out = do
  (r, w) <- createPipe
  (_, _, _, git) <- createProcess (proc "git" ["archive", "--format=tar", gitRef]) {std_out = UseHandle w}
  (_, _, _, tar) <- createProcess (proc "tar" ["-x", "-C", out]) {std_in = UseHandle r}
  a <- waitForProcess git
  b <- waitForProcess tar
  pure (a == ExitSuccess && b == ExitSuccess)

-- | Each skill becomes a plain guide: @docs/guides/<name>.md@, and each other Markdown file of
-- the skill @docs/guides/<name>-<file>.md@.
writeGuides :: FilePath -> IO ()
writeGuides out = do
  let guides = out </> "docs" </> "guides"
  createDirectoryIfMissing True guides
  dirs <- sortedIn (out </> "verbs")
  forM_ dirs $ \dir -> do
    let skill = dir </> "SKILL.md"
    has <- doesFileExist skill
    when has $ do
      text <- readFile skill
      _ <- evaluate (length text)
      let name = takeFileName dir
          body =
            replaceAll
              "If the `ptbr-writer` skill is available, apply it, and stop reading here. Use level `estrito`\nfor alert messages and runbooks, and level `descritivo` for README and wiki pages. The rules below\nare the fallback when that skill is not installed."
              "Use level `estrito` for alert messages and runbooks, and level `descritivo` for README and\nwiki pages."
              (dropWhile (`elem` (" \t\n" :: String)) (afterSecond "---\n" text))
      writeFile (guides </> (name <> ".md")) (replaceWord "the user" "the reviewer" body)
      extras <- sortedIn dir
      forM_ extras $ \e -> do
        let en = takeFileName e
        when (".md" `isSuffixOf` en && en /= "SKILL.md") $ copyFile e (guides </> (name <> "-" <> en))
  where
    afterSecond sep t = case breakOn sep t of
      Just (_, r1) -> case breakOn sep r1 of
        Just (_, r2) -> r2
        Nothing -> ""
      Nothing -> ""

-- | The text rewrites: the agent names and the skill vocabulary become plain names.
plain :: String -> String
plain t0 =
  let t1 =
        foldl
          (\t (a, b) -> replaceAll a b t)
          t0
          [ ("GROUND_TRUTH.md", "CONTRIBUTING.md"),
            ("https://github.com/h0ffmann/ptbr-writer/blob/main/skills/bipr/SKILL.md", "https://github.com/h0ffmann/ptbr-writer"),
            ("`verbs/open-pr/footer.md`", "`docs/guides/open-pr-footer.md`"),
            ("path = \"verbs/open-pr/**\"", "path = \"docs/guides/open-pr*.md\""),
            ("path = \"verbs/figure/**\"", "path = \"docs/guides/figure.md\""),
            (", `Co-Authored-By:`)", ")")
          ]
      t2 = verbDir t1
      t3 =
        foldl
          (\t (a, b) -> replaceAll a b t)
          t2
          [ ("`verbs/`", "`docs/guides/`"),
            ("the citation rules for agents", "the citation rules"),
            ("as regras de citação para agentes", "as regras de citação"),
            ("# Agents: how to cite alerts", "# How to cite alerts"),
            ("# - Never put a citation in a skill body (verbs/**): skills are loaded into other\n#   people's context. Credits go in", "# - Credits go in")
          ]
   in replaceWord "Skill" "Guide" (replaceWord "skill" "guide" (replaceWord "skills" "guides" t3))

-- | @`verbs/<name>/`@ becomes @`docs/guides/<name>.md`@.
verbDir :: String -> String
verbDir s = case s of
  [] -> []
  _
    | Just rest <- stripPrefix' "`verbs/" s,
      (name, '/' : '`' : after) <- span (\c -> isAsciiLower c || c == '-') rest,
      not (null name) ->
        "`docs/guides/" <> name <> ".md`" <> verbDir after
  c : cs -> c : verbDir cs

stripPrefix' :: String -> String -> Maybe String
stripPrefix' p s = if p `isPrefixOf` s then Just (drop (length p) s) else Nothing

isWord :: Char -> Bool
isWord c = isAlphaNum c || c == '_'

-- | Replaces every whole-word occurrence (as the @\\b@ of a regex).
replaceWord :: String -> String -> String -> String
replaceWord w r = go Nothing
  where
    go _ [] = []
    go prev s@(c : cs)
      | maybe True (not . isWord) prev,
        w `isPrefixOf` s,
        let after = drop (length w) s,
        case after of a : _ -> not (isWord a); [] -> True =
          r <> go (Just (last w)) after
      | otherwise = c : go (Just c) cs

replaceAll :: String -> String -> String -> String
replaceAll a b = go
  where
    go [] = []
    go s@(c : cs)
      | a `isPrefixOf` s = b <> go (drop (length a) s)
      | otherwise = c : go cs

breakOn :: String -> String -> Maybe (String, String)
breakOn sep = go []
  where
    go _ [] = Nothing
    go acc s@(c : cs)
      | sep `isPrefixOf` s = Just (reverse acc, drop (length sep) s)
      | otherwise = go (c : acc) cs

-- | True when a line names an AI tool: the trace check must name what it looks for. scripts/ is
-- the only place in the repo that names an AI product, and the export drops scripts/.
traceIn :: String -> Bool
traceIn line =
  any (`isInfixOf` l) ["claude", "anthropic", "opencode", "agents.md", "ground_truth", "co-authored-by", "skill.md"]
    || wordAt "ai agent" pluralEnd
    || wordAt "skill" pluralEnd
    || wordAt "verbs/" (const True)
  where
    l = map toLower line
    -- The word ends here, or after one "s": the "s?\\b" of a regex.
    pluralEnd a = case a of
      [] -> True
      's' : r -> ends r || ends a
      _ -> ends a
    ends a = case a of
      [] -> True
      c : _ -> not (isWord c)
    wordAt w endOk = go Nothing l
      where
        go _ [] = False
        go prev s@(c : cs)
          | maybe True (not . isWord) prev && w `isPrefixOf` s && endOk (drop (length w) s) = True
          | otherwise = go (Just c) cs

-- | Fails with every path and line that still names an AI tool.
check :: FilePath -> IO (Either String ())
check out = do
  files <- textFiles out
  found <- forM files $ \p -> do
    let rel = makeRelative out p
    t <- readText p
    let inPath = [rel <> ": path" | traceIn rel]
        inText = [rel <> ":" <> show n <> ": " <> trim' ln | Just s <- [t], (n, ln) <- zip [1 :: Int ..] (lines s), traceIn ln]
    pure (inPath <> inText)
  pure $ case concat found of
    [] -> Right ()
    xs -> Left ("export: AI trace remains:\n" <> unlines' xs)
  where
    trim' = reverse . dropWhile (== ' ') . reverse . dropWhile (== ' ')

-- | The text of a file, or 'Nothing' when it is not UTF-8 text.
readText :: FilePath -> IO (Maybe String)
readText p = do
  (code, out, _) <- readProcessWithExitCode "iconv" ["-f", "UTF-8", "-t", "UTF-8", p] ""
  pure (if code == ExitSuccess && '\0' `notElem` out then Just out else Nothing)

-- | Every regular file under a root, sorted, without @.git@, the licenses and symlinks.
textFiles :: FilePath -> IO [FilePath]
textFiles root = sort <$> go root
  where
    go dir = do
      entries <- sortedIn dir
      fmap concat . forM entries $ \p -> do
        let name = takeFileName p
        link <- pathIsSymbolicLink p
        isDir <- doesDirectoryExist p
        isFile <- doesFileExist p
        if link
          then pure []
          else
            if isDir && name /= ".git" && name /= "LICENSES"
              then go p
              else pure [p | isFile, not ("LICENSE" `isPrefixOf` name)]

sortedIn :: FilePath -> IO [FilePath]
sortedIn dir = do
  ok <- doesDirectoryExist dir
  if ok then map (dir </>) . sort <$> listDirectory dir else pure []

edit :: FilePath -> (String -> String) -> IO ()
edit p f = do
  ok <- doesFileExist p
  when ok $ do
    t <- readFile p
    _ <- evaluate (length t)
    writeFile p (f t)

lines' :: String -> [String]
lines' = lines

unlines' :: [String] -> String
unlines' = unlines

-- | Removes the recipe whose comment starts with a text: from it to the blank line before the
-- next comment line, or to the end of the file.
cutRecipe :: String -> String -> String
cutRecipe t start = case breakOn start t of
  Nothing -> t
  Just (before, _) ->
    let rest = drop (length before) t
     in case breakOn "\n\n# " rest of
          Just (_, after) -> before <> "# " <> after
          Nothing -> reverse (dropWhile (== '\n') (reverse before)) <> "\n"

-- | Removes the bullet that starts with a text, up to the next bullet, blank line or heading.
cutBullet :: String -> String -> String
cutBullet t start = concat (go False (splitInclusive t))
  where
    go _ [] = []
    go cutting (l : ls)
      | start `isPrefixOf` l = go True ls
      | cutting && ("- " `isPrefixOf` l || l == "\n" || "## " `isPrefixOf` l) = l : go False ls
      | cutting = go True ls
      | otherwise = l : go False ls
    splitInclusive s = case break (== '\n') s of
      (a, '\n' : r) -> (a <> "\n") : splitInclusive r
      (a, _) -> [a | not (null a)]
