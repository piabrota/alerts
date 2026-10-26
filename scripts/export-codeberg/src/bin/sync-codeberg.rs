//! Push the Codeberg copy of smart-alerts: `just sync-codeberg [flags]`.
//!
//! Builds the clean tree of a git ref (as `export-codeberg` does) on top of the Codeberg branch,
//! commits it with a plain message as the author, checks the commit for AI traces and pushes it.
//! The commit holds only the Codeberg history plus the new tree: this repo's history never leaves.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use export_codeberg::{Result, check, check_text, export};

const USAGE: &str = "usage: sync-codeberg [--remote <url>] [--branch <name>] [--ref <git-ref>]
                     [--author \"Name <email>\"] [--message <text>] [--sign] [--dry-run]

  --remote   default https://codeberg.org/piabrota/ialerts.git
  --branch   default main
  --ref      the commit of this repo to copy, default HEAD
  --author   default user.name and user.email of this repo
  --message  default \"Sync <short sha of the ref>\"
  --sign     sign the commit with your git signing setup (off by default)
  --dry-run  build and commit, but do not push; keeps the work directory";

#[derive(Debug)]
struct Args {
    remote: String,
    branch: String,
    git_ref: String,
    author: Option<String>,
    message: Option<String>,
    sign: bool,
    dry_run: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = parse(std::env::args().skip(1))?;
    let (name, email) = match &args.author {
        Some(a) => split_author(a)?,
        None => (
            git(None, &["config", "user.name"])?,
            git(None, &["config", "user.email"])?,
        ),
    };
    let sha = git(None, &["rev-parse", "--short", &args.git_ref])?;
    let message = args.message.clone().unwrap_or(format!("Sync {sha}"));
    check_text("author", &format!("{name} <{email}>"))?;
    check_text("commit message", &message)?;

    let work = std::env::temp_dir().join(format!("sync-codeberg-{}", std::process::id()));
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    git(Some(&work), &["init", "-q", "-b", &args.branch])?;
    let tip = git(None, &["ls-remote", "--heads", &args.remote, &args.branch])?;
    if tip.is_empty() {
        println!(
            "sync: {} has no branch {}: first commit",
            args.remote, args.branch
        );
    } else {
        git(
            Some(&work),
            &["fetch", "-q", "--depth", "1", &args.remote, &args.branch],
        )?;
        git(Some(&work), &["reset", "-q", "--hard", "FETCH_HEAD"])?;
    }
    clear(&work)?;
    export(&work, &args.git_ref)?;
    check(&work)?;
    git(Some(&work), &["add", "-A"])?;
    if git_ok(&work, &["diff", "--cached", "--quiet"])? {
        println!("sync: {} already holds {sha}", args.remote);
        return fs::remove_dir_all(&work).map_err(Into::into);
    }
    commit(&work, &name, &email, &message, args.sign)?;
    // A global hook or template could still add a trailer: check what git wrote.
    check_text(
        "commit",
        &git(
            Some(&work),
            &["log", "-1", "--format=%an <%ae>%n%cn <%ce>%n%B"],
        )?,
    )?;
    if args.dry_run {
        println!(
            "sync: dry run, not pushed; the commit is in {}",
            work.display()
        );
        return Ok(());
    }
    git(
        Some(&work),
        &[
            "push",
            "-q",
            &args.remote,
            &format!("HEAD:refs/heads/{}", args.branch),
        ],
    )?;
    fs::remove_dir_all(&work)?;
    println!("sync: pushed {sha} to {} ({})", args.remote, args.branch);
    Ok(())
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args> {
    let mut a = Args {
        remote: "https://codeberg.org/piabrota/ialerts.git".into(),
        branch: "main".into(),
        git_ref: "HEAD".into(),
        author: None,
        message: None,
        sign: false,
        dry_run: false,
    };
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))
        };
        match flag.as_str() {
            "--remote" => a.remote = value()?,
            "--branch" => a.branch = value()?,
            "--ref" => a.git_ref = value()?,
            "--author" => a.author = Some(value()?),
            "--message" => a.message = Some(value()?),
            "--sign" => a.sign = true,
            "--dry-run" => a.dry_run = true,
            _ => return Err(USAGE.into()),
        }
    }
    Ok(a)
}

/// `"Name <email>"` to `("Name", "email")`.
fn split_author(a: &str) -> Result<(String, String)> {
    let (name, rest) = a
        .split_once('<')
        .ok_or_else(|| format!("--author must be \"Name <email>\", not {a}"))?;
    let email = rest
        .strip_suffix('>')
        .ok_or_else(|| format!("--author must be \"Name <email>\", not {a}"))?;
    Ok((name.trim().to_string(), email.trim().to_string()))
}

/// Removes everything in `dir` except `.git`.
fn clear(dir: &Path) -> Result<()> {
    for e in fs::read_dir(dir)?.flatten() {
        let p: PathBuf = e.path();
        if e.file_name() == ".git" {
            continue;
        }
        if e.file_type()?.is_dir() {
            fs::remove_dir_all(&p)?;
        } else {
            fs::remove_file(&p)?;
        }
    }
    Ok(())
}

/// The author is also the committer. Hooks are off, so no hook adds a trailer.
fn commit(dir: &Path, name: &str, email: &str, message: &str, sign: bool) -> Result<()> {
    let status = Command::new("git")
        .current_dir(dir)
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(["-c", &format!("commit.gpgsign={sign}")])
        .args([
            "commit",
            "-q",
            "--no-verify",
            "--cleanup=strip",
            "-m",
            message,
        ])
        .env("GIT_AUTHOR_NAME", name)
        .env("GIT_AUTHOR_EMAIL", email)
        .env("GIT_COMMITTER_NAME", name)
        .env("GIT_COMMITTER_EMAIL", email)
        .status()?;
    if !status.success() {
        return Err("sync: git commit failed".into());
    }
    Ok(())
}

/// Runs git (in `dir`, or in the current directory) and returns its trimmed output.
fn git(dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let out = cmd.args(args).output()?;
    if !out.status.success() {
        return Err(format!(
            "sync: git {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Runs git in `dir` and returns whether it exited with 0.
fn git_ok(dir: &Path, args: &[&str]) -> Result<bool> {
    Ok(Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()?
        .success())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn split_author_takes_name_and_email() {
        assert_eq!(
            split_author("M.Hoffmann <hoffmann@poli.ufrj.br>").unwrap(),
            (
                "M.Hoffmann".to_string(),
                "hoffmann@poli.ufrj.br".to_string()
            )
        );
        assert!(split_author("M.Hoffmann").is_err());
    }

    #[test]
    fn parse_keeps_the_defaults() {
        let a = parse(["--dry-run".to_string()].into_iter()).unwrap();
        assert_eq!(a.remote, "https://codeberg.org/piabrota/ialerts.git");
        assert!(a.dry_run && !a.sign);
    }
}
