//! Build the Codeberg copy of alerts with no trace of AI tools.
//!
//! Usage: `just export-codeberg <out-dir> [<git-ref>]`, or `export-codeberg --check <dir>` to run
//! only the trace check on a built tree (the site that `just deploy-site` publishes).
//! `just sync-codeberg` builds the copy and pushes it.

use std::path::Path;

const USAGE: &str = "usage: export-codeberg <out-dir> [<git-ref>]\n       export-codeberg --check <dir>   (only the trace check, for a built site)";

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> export_codeberg::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--check" => {
            export_codeberg::check(Path::new(dir))?;
            println!("export: no AI trace in {dir}");
        }
        [out] => export(out, "HEAD")?,
        [out, r] => export(out, r)?,
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn export(out: &str, git_ref: &str) -> export_codeberg::Result<()> {
    export_codeberg::export(Path::new(out), git_ref)?;
    println!("export: clean tree of {git_ref} in {out}");
    Ok(())
}
