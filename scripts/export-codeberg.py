#!/usr/bin/env python3
"""Build the Codeberg copy of smart-alerts with no trace of AI tools.

Usage: python3 -I scripts/export-codeberg.py <out-dir> [<git-ref>]

Writes the tree of <git-ref> (default HEAD) into <out-dir>, without the agent files, and with
GROUND_TRUTH.md as CONTRIBUTING.md and each skill in verbs/ as docs/guides/<verb>.md. The script fails when a
trace remains. It never writes git history: commit the result with a plain message and no
co-author trailer.
"""

import io
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

DROP = ["verbs", "scripts", "opencode.json", ".mcp.json"]
# The trace check must name what it looks for. scripts/ is the only place in the repo that names
# an AI product (here and in link-verbs.sh), and the export drops scripts/.
TRACE = re.compile(r"claude|anthropic|opencode|AGENTS\.md|GROUND_TRUTH|\bAI agents?\b|co-authored-by|"
                   r"SKILL\.md|\bskills?\b|\bverbs/", re.IGNORECASE)


def main() -> None:
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    out = Path(sys.argv[1]).resolve()
    ref = sys.argv[2] if len(sys.argv) == 3 else "HEAD"
    if out.exists() and any(p.name != ".git" for p in out.iterdir()):
        sys.exit(f"export: {out} must be empty (a .git directory is allowed)")
    out.mkdir(parents=True, exist_ok=True)

    archive = subprocess.run(["git", "archive", "--format=tar", ref], check=True,
                             capture_output=True).stdout
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(out, filter="data")

    # Each skill becomes a plain guide: docs/guides/<name>.md.
    guides = out / "docs" / "guides"
    guides.mkdir(parents=True, exist_ok=True)
    for skill in sorted((out / "verbs").glob("*/SKILL.md")):
        body = skill.read_text(encoding="utf-8").split("---\n", 2)[2].lstrip()
        body = body.replace(
            "If the `ptbr-writer` skill is available, apply it, and stop reading here. Use level `estrito`\n"
            "for alert messages and runbooks, and level `descritivo` for README and wiki pages. The rules below\n"
            "are the fallback when that skill is not installed.",
            "Use level `estrito` for alert messages and runbooks, and level `descritivo` for README and\n"
            "wiki pages.")
        body = re.sub(r"\bthe user\b", "the reviewer", body)
        (guides / f"{skill.parent.name}.md").write_text(body, encoding="utf-8")
        for extra in sorted(skill.parent.glob("*.md")):
            if extra.name != "SKILL.md":
                shutil.copy(extra, guides / f"{skill.parent.name}-{extra.name}")

    for name in DROP:
        p = out / name
        if p.is_dir():
            shutil.rmtree(p)
        elif p.exists():
            p.unlink()
    justfile = out / "justfile"
    justfile.write_text(re.sub(r"# Zip each skill in verbs/.*?\n\n(?=# )", "",
                               justfile.read_text(encoding="utf-8"), count=1, flags=re.S),
                        encoding="utf-8")

    gitignore = out / ".gitignore"
    gitignore.write_text(re.sub(r"\n# Agent instructions live in GROUND_TRUTH\.md\nAGENTS\.md\n", "\n",
                                gitignore.read_text(encoding="utf-8")), encoding="utf-8")

    (out / "GROUND_TRUTH.md").rename(out / "CONTRIBUTING.md")
    contributing = out / "CONTRIBUTING.md"
    text = contributing.read_text(encoding="utf-8")
    text = re.sub(r"\A# GROUND_TRUTH\.md\n\n.*?\n\n", "# Contributing\n\nRules for everyone who works on "
                  "`smart-alerts`.\n\n", text, count=1, flags=re.S)
    text = re.sub(r"^- \*\*Codeberg mirror without AI trace:\*\*.*?(?=^- |^\n|^## )", "", text,
                  flags=re.M | re.S)
    text = text.replace("MCP server (rmcp, stdio) for agents. Configured in `.mcp.json` and "
                        "`opencode.json`.", "MCP server (rmcp, stdio): `just mcp`.")
    contributing.write_text(text, encoding="utf-8")

    # Text files: the agent names and the skill vocabulary become plain names.
    for p in sorted(out.rglob("*")):
        if not p.is_file() or p.name.startswith("LICENSE") or "LICENSES" in p.parts:
            continue
        try:
            t = p.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        t = t.replace("GROUND_TRUTH.md", "CONTRIBUTING.md")
        t = t.replace("https://github.com/h0ffmann/ptbr-writer/blob/main/skills/bipr/SKILL.md",
                      "https://github.com/h0ffmann/ptbr-writer")
        t = t.replace("`verbs/open-pr/footer.md`", "`docs/guides/open-pr-footer.md`")
        t = t.replace('path = "verbs/open-pr/**"', 'path = "docs/guides/open-pr*.md"')
        t = t.replace('path = "verbs/figure/**"', 'path = "docs/guides/figure.md"')
        t = t.replace(", `Co-Authored-By:`)", ")")
        t = re.sub(r"`verbs/([a-z-]+)/`", r"`docs/guides/\1.md`", t)
        t = re.sub(r"`verbs/`", "`docs/guides/`", t)
        t = t.replace("the citation rules for agents", "the citation rules")
        t = t.replace("as regras de citação para agentes", "as regras de citação")
        t = t.replace("# Agents: how to cite smart-alerts", "# How to cite smart-alerts")
        t = t.replace("# - Never put a citation in a skill body (verbs/**): skills are loaded into other\n"
                      "#   people's context. Credits go in", "# - Credits go in")
        t = re.sub(r"\bskills\b", "guides", t)
        t = re.sub(r"\bskill\b", "guide", t)
        t = re.sub(r"\bSkill\b", "Guide", t)
        p.write_text(t, encoding="utf-8")

    found = []
    for p in sorted(out.rglob("*")):
        if ".git" in p.relative_to(out).parts or not p.is_file() or p.name.startswith("LICENSE") \
                or "LICENSES" in p.parts:
            continue
        if TRACE.search(str(p.relative_to(out))):
            found.append(f"{p.relative_to(out)}: path")
        try:
            lines = p.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        found += [f"{p.relative_to(out)}:{i}: {l.strip()}" for i, l in enumerate(lines, 1)
                  if TRACE.search(l)]
    if found:
        sys.exit("export: AI trace remains:\n" + "\n".join(found))
    print(f"export: clean tree of {ref} in {out}")


if __name__ == "__main__":
    main()
