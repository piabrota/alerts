//! How every statement of `docs/proofs/` is proved: by refutation, as in the z3 work of
//! [h0ffmann/proof](https://github.com/h0ffmann/proof) (`z3/refute.py`).
//!
//! A statement "premises imply conclusion" holds when z3 finds no model of "premises and not
//! conclusion": the answer `unsat`. A model, when there is one, is a counterexample, and
//! `proofs --prove` prints it. Some statements expect a counterexample (`expect: sat`): they show
//! that a hypothesis is needed, or that the code allows more than its comment says. Each one says
//! why.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

/// What z3 must answer for a statement.
#[derive(Debug, Clone, Copy)]
pub enum Expect {
    /// No counterexample: the statement holds.
    Unsat,
    /// A counterexample must exist; the text says what it shows.
    Sat(&'static str),
}

/// One statement about the code: its name, the code it is about (`file:line`), what it says,
/// and its SMT-LIB body (the premises and the negated conclusion, as `assert`s).
#[derive(Debug)]
pub struct Statement {
    pub name: &'static str,
    pub code: &'static str,
    pub says: &'static str,
    pub smt: String,
    pub expect: Expect,
}

/// A file of statements that share one prelude of definitions.
#[derive(Debug)]
pub struct Theory {
    pub title: &'static str,
    pub prelude: String,
    pub statements: Vec<Statement>,
}

/// The theory as an SMT-LIB 2 file. Each statement sits in its own `(push)/(pop)` block, under
/// a comment that names it, so `proofs --prove` can run it alone and name its answer.
pub fn render(t: &Theory) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "; {}", t.title);
    s.push_str("; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.\n");
    s.push_str("; Each statement is asserted negated: `unsat` proves it. A statement marked\n");
    s.push_str(
        "; `expect: sat` must have a counterexample, and the model is that counterexample.\n\n",
    );
    s.push_str(&t.prelude);
    for st in &t.statements {
        let _ = writeln!(s, "\n; {} ({}): {}", st.name, st.code, st.says);
        if let Expect::Sat(why) = st.expect {
            let _ = writeln!(s, "; expect: sat ({why})");
        }
        s.push_str("(push)\n");
        s.push_str(&st.smt);
        if !st.smt.ends_with('\n') {
            s.push('\n');
        }
        s.push_str("(check-sat)\n(pop)\n");
    }
    s
}

/// One `(push)/(pop)` block of a committed `.smt2` file, with the definitions before it.
#[derive(Debug)]
struct Block {
    name: String,
    expect_sat: bool,
    /// The definitions before the block, then the block.
    script: String,
    /// The block alone: the names a counterexample shows are the names it uses.
    body: String,
}

/// Reads the blocks of an SMT-LIB file written by this crate. The name of a block is the first
/// word of the last comment before its `(push)` (`c1:` gives `c1`); `; expect: sat` in that
/// comment marks a block that must have a counterexample.
fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut prelude = String::new();
    let mut comments: Vec<&str> = Vec::new();
    let mut in_comment = false;
    let mut body: Option<String> = None;
    for line in text.lines() {
        if let Some(b) = body.as_mut() {
            if line.trim() == "(pop)" {
                let head = comments.first().copied().unwrap_or("");
                let name = head
                    .trim_start_matches(';')
                    .trim()
                    .split([' ', ':'])
                    .next()
                    .unwrap_or("")
                    .to_owned();
                let expect_sat = comments
                    .iter()
                    .any(|c| c.trim_start_matches(';').trim().starts_with("expect: sat"));
                out.push(Block {
                    name,
                    expect_sat,
                    script: format!("{prelude}{b}"),
                    body: b.clone(),
                });
                body = None;
                comments.clear();
            } else {
                b.push_str(line);
                b.push('\n');
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed == "(push)" {
            body = Some(String::new());
        } else if trimmed.starts_with(';') {
            if !in_comment {
                comments.clear();
            }
            in_comment = true;
            comments.push(line);
            prelude.push_str(line);
            prelude.push('\n');
        } else {
            in_comment = false;
            prelude.push_str(line);
            prelude.push('\n');
        }
    }
    out
}

fn z3(script: &str) -> Result<String, String> {
    let bin = std::env::var("Z3").unwrap_or_else(|_| "z3".to_owned());
    let mut child = Command::new(&bin)
        .arg("-in")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {bin}: {e}; z3 is in the dev shell (`nix develop`)"))?;
    child
        .stdin
        .take()
        .ok_or("z3: no stdin")?
        .write_all(script.as_bytes())
        .map_err(|e| format!("z3: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("z3: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The entries of a model whose names the block uses, as `name = value`: the counterexample.
fn counterexample(model: &str, body: &str) -> String {
    let uses = |name: &str| {
        body.match_indices(name).any(|(i, _)| {
            let word = |c: char| c.is_alphanumeric() || "-_!.".contains(c);
            let before = body[..i].chars().next_back().is_none_or(|c| !word(c));
            let after = body[i + name.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c));
            before && after
        })
    };
    // The top-level entries of `( (define-fun ...) (define-fun ...) )`.
    let inner = model
        .trim()
        .strip_prefix('(')
        .and_then(|m| m.strip_suffix(')'));
    let (mut entries, mut depth, mut start) = (Vec::new(), 0usize, 0usize);
    for (i, c) in inner.unwrap_or("").char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    entries.push(&inner.unwrap_or("")[start..=i]);
                }
            }
            _ => {}
        }
    }
    let shown: Vec<String> = entries
        .iter()
        .filter_map(|e| {
            let e = e.split_whitespace().collect::<Vec<_>>().join(" ");
            let rest = e
                .strip_prefix("(define-fun ")?
                .strip_suffix(')')?
                .to_owned();
            let (name, rest) = rest.split_once(' ')?;
            match rest.strip_prefix("() ") {
                // A constant: `name () Sort value`, where the sort may be `(_ BitVec 27)`.
                Some(sv) => (uses(name) && !sv.starts_with("RegLan"))
                    .then(|| format!("{name} = {}", after_sort(sv))),
                // A function: shown whole, since the block may reach it through a definition.
                None => Some(format!("{name} {rest}")),
            }
        })
        .collect();
    one_line(&shown.join(", "))
}

/// The value after the sort of `Sort value` or `(_ BitVec 27) value`.
fn after_sort(sv: &str) -> &str {
    if sv.starts_with('(') {
        let mut depth = 0usize;
        for (i, c) in sv.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return sv[i + 1..].trim_start();
                    }
                }
                _ => {}
            }
        }
        sv
    } else {
        sv.split_once(' ').map_or(sv, |(_, v)| v)
    }
}

/// A text on one line, cut to a length a terminal shows.
fn one_line(model: &str) -> String {
    let s = model.split_whitespace().collect::<Vec<_>>().join(" ");
    match s.char_indices().nth(400) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s,
    }
}

/// Runs z3 on every block of every `.smt2` file in `dir`: one line per statement, `OK` or
/// `FAIL`, and an error when one failed.
pub fn prove(dir: &Path) -> Result<(), String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "smt2"))
        .collect();
    files.sort();
    let (mut checked, mut failed) = (0usize, 0usize);
    for path in files {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for b in blocks(&text) {
            let start = Instant::now();
            let out = z3(&b.script)?;
            let answer = out.lines().next().unwrap_or("").trim().to_owned();
            let error = out.contains("(error");
            let holds = !error && answer == if b.expect_sat { "sat" } else { "unsat" };
            let note = if error {
                format!("z3 error: {}", one_line(&out))
            } else if answer == "sat" {
                let model = z3(&format!("{}(get-model)\n", b.script))?;
                let model = model.split_once('\n').map_or("", |(_, m)| m);
                let shown = counterexample(model, &b.body);
                if b.expect_sat {
                    format!("counterexample, as expected: {shown}")
                } else {
                    format!("counterexample: {shown}")
                }
            } else if answer == "unsat" && b.expect_sat {
                "no counterexample, but one was expected".to_owned()
            } else if answer == "unsat" {
                "no counterexample (unsat)".to_owned()
            } else {
                format!("z3 answered {answer:?}")
            };
            println!(
                "{}  {file:<14} {:<34} {:>5.1} s  {note}",
                if holds { "OK  " } else { "FAIL" },
                b.name,
                start.elapsed().as_secs_f64(),
            );
            checked += 1;
            if !holds {
                failed += 1;
            }
        }
    }
    println!();
    if failed > 0 {
        return Err(format!("prove: {failed} of {checked} statements failed"));
    }
    println!("prove: all {checked} statements hold");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_carry_name_expectation_and_prelude() {
        let t = Theory {
            title: "T: test",
            prelude: "(declare-const x Int)\n".into(),
            statements: vec![
                Statement {
                    name: "pos",
                    code: "a.rs:1",
                    says: "x*x is not negative",
                    smt: "(assert (< (* x x) 0))".into(),
                    expect: Expect::Unsat,
                },
                Statement {
                    name: "neg",
                    code: "a.rs:2",
                    says: "x is positive",
                    smt: "(assert (not (> x 0)))".into(),
                    expect: Expect::Sat("x can be 0"),
                },
            ],
        };
        let b = blocks(&render(&t));
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].name.as_str(), b[0].expect_sat), ("pos", false));
        assert_eq!((b[1].name.as_str(), b[1].expect_sat), ("neg", true));
        assert!(b[1].script.contains("(declare-const x Int)"));
        assert!(b[1].script.contains("(check-sat)"));
        assert!(!b[1].script.contains("(< (* x x) 0)"));
    }

    #[test]
    fn a_counterexample_shows_the_names_its_block_uses() {
        let model = "(\n  (define-fun y () Int\n    0)\n  (define-fun s () Int\n    60)\n  (define-fun p ((x!0 Int)) Bool\n    false)\n)";
        assert_eq!(
            counterexample(model, "(assert (> s 59))"),
            "s = 60, p ((x!0 Int)) Bool false"
        );
        let bv = "((define-fun S () (_ BitVec 3) #b101))";
        assert_eq!(counterexample(bv, "(assert (= S S))"), "S = #b101");
    }

    #[test]
    fn old_style_blocks_are_named_by_their_comment() {
        let b = blocks(
            "(declare-const x Int)\n\n; c1: x is x.\n(push)\n(assert (distinct x x))\n(check-sat)\n(pop)\n",
        );
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].name, "c1");
        assert!(!b[0].expect_sat);
        // A comment, then declarations, then the block: the comment still names it.
        let b = blocks(
            "; c2: parity.\n(declare-fun h (Int) Bool)\n(push)\n(assert false)\n(check-sat)\n(pop)\n",
        );
        assert_eq!(b[0].name, "c2");
    }
}
