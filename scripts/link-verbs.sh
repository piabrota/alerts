#!/usr/bin/env bash
# Make a local coding agent read GROUND_TRUTH.md and the skills in verbs/.
# The links are local only: this script lists them in .git/info/exclude, never in .gitignore.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
agent_dir=".claude"          # the directory that the local agent reads
mkdir -p "$agent_dir"
ln -sfn ../verbs "$agent_dir/skills"
printf '@GROUND_TRUTH.md\n' > CLAUDE.md
exclude=.git/info/exclude
for path in "/$agent_dir/" /CLAUDE.md; do
    grep -qxF "$path" "$exclude" 2>/dev/null || printf '%s\n' "$path" >> "$exclude"
done
echo "linked: $agent_dir/skills -> verbs, CLAUDE.md -> GROUND_TRUTH.md"
