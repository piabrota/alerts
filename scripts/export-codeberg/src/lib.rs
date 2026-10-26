//! The Codeberg copy of smart-alerts, with no trace of AI tools.
//!
//! [`export`] writes the tree of a git ref into a directory, without the agent files, with
//! GROUND_TRUTH.md as CONTRIBUTING.md and each skill in verbs/ as `docs/guides/<verb>.md`. It
//! fails when a trace remains. It never writes git history: `sync-codeberg` commits the result
//! with a plain message as the author.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DROP: [&str; 4] = ["verbs", "scripts", "opencode.json", ".mcp.json"];
/// Skills that become no guide: they are about the copy itself.
const NO_GUIDE: [&str; 1] = ["sync-codeberg"];
/// The trace check must name what it looks for. scripts/ is the only place in the repo that names
/// an AI product, and the export drops scripts/.
const TRACE: &str = r"(?i)claude|anthropic|opencode|AGENTS\.md|GROUND_TRUTH|\bAI agents?\b|co-authored-by|SKILL\.md|\bskills?\b|\bverbs/";

/// Writes the clean tree of `git_ref` into `out` (empty, or only `.git`).
pub fn export(out: &Path, git_ref: &str) -> Result<()> {
    let out = std::path::absolute(out)?;
    if out.exists()
        && fs::read_dir(&out)?
            .flatten()
            .any(|e| e.file_name() != ".git")
    {
        return Err(format!(
            "export: {} must be empty (a .git directory is allowed)",
            out.display()
        )
        .into());
    }
    fs::create_dir_all(&out)?;
    extract(git_ref, &out)?;
    write_guides(&out)?;
    for name in DROP {
        let p = out.join(name);
        if p.is_dir() {
            fs::remove_dir_all(&p)?;
        } else if p.symlink_metadata().is_ok() {
            fs::remove_file(&p)?;
        }
    }
    // The recipes lose the export, the sync and the lines that run them: scripts/ is not in the
    // copy.
    edit(&out.join("justfile"), |t| {
        [
            "# Zip each skill in verbs/",
            "# Build the Codeberg copy",
            "# Sync the Codeberg copy",
        ]
        .iter()
        .fold(t.to_string(), |t, start| cut_recipe(&t, start))
        .split_inclusive('\n')
        .filter(|l| !l.contains("scripts/export-codeberg/"))
        .collect()
    })?;
    edit(&out.join(".gitignore"), |t| {
        t.replace(
            "\n# Agent instructions live in GROUND_TRUTH.md\nAGENTS.md\n",
            "\n",
        )
    })?;
    fs::rename(out.join("GROUND_TRUTH.md"), out.join("CONTRIBUTING.md"))?;
    let header = Regex::new(r"(?s)\A# GROUND_TRUTH\.md\n\n.*?\n\n")?;
    edit(&out.join("CONTRIBUTING.md"), |t| {
        let t = header.replace(
            t,
            "# Contributing\n\nRules for everyone who works on `smart-alerts`.\n\n",
        );
        cut_bullet(&t, "- **Codeberg mirror without AI trace:**").replace(
            "MCP server (rmcp, stdio) for agents. Configured in `.mcp.json` and `opencode.json`.",
            "MCP server (rmcp, stdio): `just mcp`.",
        )
    })?;
    let plain = Plain::new()?;
    for p in text_files(&out)? {
        if let Ok(t) = fs::read_to_string(&p) {
            fs::write(&p, plain.apply(&t))?;
        }
    }
    check(&out)
}

/// Fails when `text` names an AI tool: for a commit message or an author.
pub fn check_text(what: &str, text: &str) -> Result<()> {
    if Regex::new(TRACE)?.is_match(text) {
        return Err(format!("export: AI trace in the {what}: {text}").into());
    }
    Ok(())
}

/// `git archive <ref> | tar -x -C <out>`.
fn extract(git_ref: &str, out: &Path) -> Result<()> {
    let mut archive = Command::new("git")
        .args(["archive", "--format=tar", git_ref])
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = archive.stdout.take().ok_or("git archive: no output")?;
    let tar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(out)
        .stdin(stdout)
        .status()?;
    if !archive.wait()?.success() || !tar.success() {
        return Err(format!("export: cannot extract {git_ref}").into());
    }
    Ok(())
}

/// Each skill becomes a plain guide: `docs/guides/<name>.md`, and each other Markdown file of
/// the skill `docs/guides/<name>-<file>.md`.
fn write_guides(out: &Path) -> Result<()> {
    let guides = out.join("docs").join("guides");
    fs::create_dir_all(&guides)?;
    let the_user = Regex::new(r"\bthe user\b")?;
    for dir in sorted(&out.join("verbs"))? {
        let skill = dir.join("SKILL.md");
        if !skill.is_file() {
            continue;
        }
        let name = file_name(&dir);
        if NO_GUIDE.contains(&name.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&skill)?;
        let body = text.splitn(3, "---\n").nth(2).unwrap_or("").trim_start().replace(
            "If the `ptbr-writer` skill is available, apply it, and stop reading here. Use level `estrito`\n\
             for alert messages and runbooks, and level `descritivo` for README and wiki pages. The rules below\n\
             are the fallback when that skill is not installed.",
            "Use level `estrito` for alert messages and runbooks, and level `descritivo` for README and\n\
             wiki pages.",
        );
        fs::write(
            guides.join(format!("{name}.md")),
            the_user.replace_all(&body, "the reviewer").as_ref(),
        )?;
        for extra in sorted(&dir)? {
            let extra_name = file_name(&extra);
            if extra_name.ends_with(".md") && extra_name != "SKILL.md" {
                fs::copy(&extra, guides.join(format!("{name}-{extra_name}")))?;
            }
        }
    }
    Ok(())
}

/// The text rewrites: the agent names and the skill vocabulary become plain names.
#[derive(Debug)]
struct Plain {
    verb_dir: Regex,
    skills: Regex,
    skill: Regex,
    skill_cap: Regex,
}

impl Plain {
    fn new() -> Result<Self> {
        Ok(Self {
            verb_dir: Regex::new(r"`verbs/([a-z-]+)/`")?,
            skills: Regex::new(r"\bskills\b")?,
            skill: Regex::new(r"\bskill\b")?,
            skill_cap: Regex::new(r"\bSkill\b")?,
        })
    }

    fn apply(&self, t: &str) -> String {
        let t = t
            .replace("GROUND_TRUTH.md", "CONTRIBUTING.md")
            .replace(
                "https://github.com/h0ffmann/ptbr-writer/blob/main/skills/bipr/SKILL.md",
                "https://github.com/h0ffmann/ptbr-writer",
            )
            .replace(
                "`verbs/open-pr/footer.md`",
                "`docs/guides/open-pr-footer.md`",
            )
            .replace(
                r#"path = "verbs/open-pr/**""#,
                r#"path = "docs/guides/open-pr*.md""#,
            )
            .replace(
                r#"path = "verbs/figure/**""#,
                r#"path = "docs/guides/figure.md""#,
            )
            .replace(", `Co-Authored-By:`)", ")");
        let t = self.verb_dir.replace_all(&t, "`docs/guides/$1.md`");
        let t = t
            .replace("`verbs/`", "`docs/guides/`")
            .replace("the citation rules for agents", "the citation rules")
            .replace("as regras de citação para agentes", "as regras de citação")
            .replace("# Agents: how to cite smart-alerts", "# How to cite smart-alerts")
            .replace(
                "# - Never put a citation in a skill body (verbs/**): skills are loaded into other\n\
                 #   people's context. Credits go in",
                "# - Credits go in",
            );
        let t = self.skills.replace_all(&t, "guides");
        let t = self.skill.replace_all(&t, "guide");
        self.skill_cap.replace_all(&t, "Guide").into_owned()
    }
}

/// Fails with every path and line that still names an AI tool.
pub fn check(out: &Path) -> Result<()> {
    let trace = Regex::new(TRACE)?;
    let mut found = vec![];
    for p in text_files(out)? {
        let rel = p.strip_prefix(out)?.display().to_string();
        if trace.is_match(&rel) {
            found.push(format!("{rel}: path"));
        }
        if let Ok(t) = fs::read_to_string(&p) {
            for (i, line) in t.lines().enumerate() {
                if trace.is_match(line) {
                    found.push(format!("{rel}:{}: {}", i + 1, line.trim()));
                }
            }
        }
    }
    if found.is_empty() {
        Ok(())
    } else {
        Err(format!("export: AI trace remains:\n{}", found.join("\n")).into())
    }
}

/// Every regular file under `root`, sorted, without `.git`, the licenses and symlinks.
fn text_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for p in sorted(&dir)? {
            let name = file_name(&p);
            let meta = p.symlink_metadata()?;
            if meta.is_dir() && name != ".git" && name != "LICENSES" {
                stack.push(p);
            } else if meta.is_file() && !name.starts_with("LICENSE") {
                files.push(p);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut v: Vec<PathBuf> = fs::read_dir(dir)?.flatten().map(|e| e.path()).collect();
    v.sort();
    Ok(v)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn edit(p: &Path, f: impl FnOnce(&str) -> String) -> Result<()> {
    let t = fs::read_to_string(p)?;
    fs::write(p, f(&t))?;
    Ok(())
}

/// Removes the recipe whose comment starts with `start`: from `start` to the blank line before
/// the next comment line, or to the end of the file.
fn cut_recipe(t: &str, start: &str) -> String {
    let Some(i) = t.find(start) else {
        return t.to_string();
    };
    match t[i..].find("\n\n# ") {
        Some(j) => format!("{}{}", &t[..i], &t[i + j + 2..]),
        None => format!("{}\n", t[..i].trim_end_matches('\n')),
    }
}

/// Removes the bullet that starts with `start`, up to the next bullet, blank line or heading.
fn cut_bullet(t: &str, start: &str) -> String {
    let mut out = String::new();
    let mut cutting = false;
    for line in t.split_inclusive('\n') {
        if line.starts_with(start) {
            cutting = true;
            continue;
        }
        if cutting && (line.starts_with("- ") || line == "\n" || line.starts_with("## ")) {
            cutting = false;
        }
        if !cutting {
            out.push_str(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_recipe_keeps_the_next_recipe() {
        let t = "a:\n    x\n\n# Zip each skill in verbs/\npack:\n    y\n\n# Next\nnext:\n";
        assert_eq!(
            cut_recipe(t, "# Zip each skill in verbs/"),
            "a:\n    x\n\n# Next\nnext:\n"
        );
        assert_eq!(cut_recipe("a:\n\n# Last\nlast:\n", "# Last"), "a:\n");
    }

    #[test]
    fn cut_bullet_stops_at_the_next_bullet() {
        let t = "- one\n- **Codeberg mirror without AI trace:** x\n  y\n- two\n";
        assert_eq!(
            cut_bullet(t, "- **Codeberg mirror without AI trace:**"),
            "- one\n- two\n"
        );
    }
}
