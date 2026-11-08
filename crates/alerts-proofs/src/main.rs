//! `proofs`: the alert domain as RDF and its invariants as SMT, from one set of facts read out
//! of `alerts-core` at run time.
//!
//! - `proofs` writes `docs/proofs/facts.ttl` and the `p*.smt2` conjectures, after running the
//!   exhaustive Rust checks.
//! - `proofs --check` (in `just check` as `just proofs-check`) runs the same checks and fails
//!   when a committed file is not the output of this code.
//! - `just prove` runs z3 on the `.smt2` files: every answer must be `unsat`.

mod checks;
mod facts;
mod smt;
mod ttl;

use std::fs;
use std::path::Path;
use std::process::ExitCode;

const OUT_DIR: &str = "docs/proofs";

fn outputs(facts: &facts::Facts) -> Vec<(&'static str, String)> {
    vec![
        ("facts.ttl", ttl::facts_ttl(facts)),
        ("p1-routing.smt2", smt::routing_smt(facts)),
        ("p2-severity.smt2", smt::severity_smt()),
        ("p3-stages.smt2", smt::stages_smt()),
    ]
}

fn run(check: bool) -> Result<(), String> {
    let facts = facts::Facts::gather();
    checks::run(&facts)?;

    let dir = Path::new(OUT_DIR);
    if check {
        for (name, want) in outputs(&facts) {
            let path = dir.join(name);
            let got = fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}; run `just proofs`", path.display()))?;
            if got != want {
                return Err(format!(
                    "{} is not the output of alerts-proofs; run `just proofs`",
                    path.display()
                ));
            }
        }
        println!("proofs-check: ok — checks hold and {OUT_DIR} matches the code");
    } else {
        fs::create_dir_all(dir).map_err(|e| format!("{OUT_DIR}: {e}"))?;
        for (name, content) in outputs(&facts) {
            let path = dir.join(name);
            fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    match run(check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("proofs: {e}");
            ExitCode::FAILURE
        }
    }
}
