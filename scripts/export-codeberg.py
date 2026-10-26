#!/usr/bin/env python3
"""Build the Codeberg copy of smart-alerts with no trace of AI tools.

Usage: python3 -I scripts/export-codeberg.py <out-dir> [<git-ref>]

Writes the tree of <git-ref> (default HEAD) into <out-dir>, without the agent files, and with
AGENTS.md as CONTRIBUTING.md and the docs skills as docs/style/<lang>.md. The script fails when a
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

DROP = ["CLAUDE.md", ".claude", "opencode.json", ".mcp.json", "scripts/export-codeberg.py"]
TRACE = re.compile(r"claude|anthropic|opencode|AGENTS\.md|\bAI agents?\b|co-authored-by|SKILL\.md",
                   re.IGNORECASE)
LANGS = ["pt-br", "en", "zh", "ja"]


def sub(path: Path, pairs: list[tuple[str, str]]) -> None:
    text = path.read_text(encoding="utf-8")
    for old, new in pairs:
        if old not in text:
            sys.exit(f"export: expected text not found in {path}: {old!r}")
        text = text.replace(old, new)
    path.write_text(text, encoding="utf-8")


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

    # The docs skills become plain style guides.
    style = out / "docs" / "style"
    style.mkdir(parents=True, exist_ok=True)
    for lang in LANGS:
        src = out / ".claude" / "skills" / f"docs-{lang}" / "SKILL.md"
        body = src.read_text(encoding="utf-8").split("---\n", 2)[2].lstrip()
        body = re.sub(r"^# docs-(\S+)", r"# Style guide: \1", body, count=1, flags=re.M)
        body = body.replace("Give the user a literal back-translation", "Give the reviewer a literal back-translation")
        body = body.replace("tell\n  the user which pages need the `docs-zh` or `docs-ja` skill.",
                            "update\n  those pages with `docs/style/zh.md` and `docs/style/ja.md`.")
        body = body.replace(
            "If the `ptbr-writer` skill is available, apply it, and stop reading here. Use level `estrito`\n"
            "for alert messages and runbooks, and level `descritivo` for README and wiki pages. The rules below\n"
            "are the fallback when that skill is not installed.",
            "Use level `estrito` for alert messages and runbooks, and level `descritivo` for README and\n"
            "wiki pages.")
        (style / f"{lang}.md").write_text(body, encoding="utf-8")

    for name in DROP:
        p = out / name
        if p.is_dir():
            shutil.rmtree(p)
        elif p.exists():
            p.unlink()
    scripts = out / "scripts"
    if scripts.is_dir() and not any(scripts.iterdir()):
        scripts.rmdir()

    contributing = out / "CONTRIBUTING.md"
    (out / "AGENTS.md").rename(contributing)
    text = contributing.read_text(encoding="utf-8")
    text = re.sub(r"\A# AGENTS\.md\n\n.*?\n\n", "# Contributing\n\nRules for everyone who works on "
                  "`smart-alerts`.\n\n", text, count=1, flags=re.S)
    text = re.sub(r"^- \*\*Codeberg mirror without AI trace:\*\*.*?(?=^- |^\n|^## )", "", text,
                  flags=re.M | re.S)
    text = text.replace("MCP server (rmcp, stdio) for agents. Configured in `.mcp.json` (Claude Code) "
                        "and `opencode.json` (OpenCode).", "MCP server (rmcp, stdio): `just mcp`.")
    text = text.replace("Four languages, each with a controlled-language skill in `.claude/skills/`:",
                        "Four languages, each with a controlled-language style guide in `docs/style/`:")
    text = text.replace("| Language | Skill |", "| Language | Style guide |")
    for lang in LANGS:
        text = text.replace(f"| `docs-{lang}` |", f"| `docs/style/{lang}.md` |")
    text = text.replace("follows `docs-pt-br` (`estrito`)", "follows `docs/style/pt-br.md` (`estrito`)")
    text = text.replace("level of `docs-pt-br`", "level of `docs/style/pt-br.md`")
    contributing.write_text(text, encoding="utf-8")

    for f in ["README.md", "README.en.md", "docs/MAROLA.md", "docs/SURVEY_NLP.md"]:
        p = out / f
        p.write_text(p.read_text(encoding="utf-8").replace("AGENTS.md", "CONTRIBUTING.md"),
                     encoding="utf-8")
    sub(out / "README.md", [("as regras de citação para agentes", "as regras de citação")])
    sub(out / "README.en.md", [("the citation rules for agents", "the citation rules")])
    sub(out / "REUSE.toml", [
        ("# Agents: how to cite smart-alerts", "# How to cite smart-alerts"),
        ("# - Never put a citation in a skill body (.claude/skills/**): skills are loaded into other\n"
         "#   people's context. Credits go in README.md and README.en.md.\n",
         "# - Credits go in README.md and README.en.md.\n"),
    ])

    found = []
    for p in sorted(out.rglob("*")):
        if ".git" in p.relative_to(out).parts or not p.is_file():
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
