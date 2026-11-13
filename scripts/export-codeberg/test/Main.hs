module Main (main) where

import Control.Monad (unless)
import Export
import System.Exit (exitFailure)

main :: IO ()
main = do
  let checks =
        [ ( "cut_recipe_keeps_the_next_recipe",
            cutRecipe "a:\n    x\n\n# Zip each skill in verbs/\npack:\n    y\n\n# Next\nnext:\n" "# Zip each skill in verbs/" == "a:\n    x\n\n# Next\nnext:\n"
          ),
          ("cut_recipe: last recipe", cutRecipe "a:\n\n# Last\nlast:\n" "# Last" == "a:\n"),
          ( "cut_bullet_stops_at_the_next_bullet",
            cutBullet "- one\n- **Codeberg mirror without AI trace:** x\n  y\n- two\n" "- **Codeberg mirror without AI trace:**" == "- one\n- two\n"
          ),
          ("trace: names", all traceIn ["Claude Code", "see AGENTS.md", "Co-Authored-By: x", "a skill", "two skills", "verbs/open-pr", "AI agents"]),
          ("trace: plain words", not (any traceIn ["skillful", "adverbs/x", "the AI agentic", "plain text"])),
          ("plain: skills become guides", plain "the skill and the skills of `verbs/open-pr/`" == "the guide and the guides of `docs/guides/open-pr.md`"),
          ("replaceWord: whole words", replaceWord "the user" "the reviewer" "the user and the users" == "the reviewer and the users")
        ]
  let failed = [n | (n, ok) <- checks, not ok]
  mapM_ (putStrLn . ("FAIL " <>)) failed
  unless (null failed) exitFailure
  putStrLn "export-codeberg: ok"
