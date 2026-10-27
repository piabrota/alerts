//! `scaffold`: writes a new resource of `alerts-crud`, as `cargo loco generate scaffold` does.
//!
//! One run writes the model, the controller and the migration, and registers the three modules
//! at the `// scaffold:` markers. The desktop interface draws the new resource from its spec, so
//! the interface needs no change.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage: scaffold <name> <field:kind>... [--root <dir>]

Writes a new alerts-crud resource. <name> is the plural snake_case table name, for example
rivers. Each field is <name>:<kind>, with one of these kinds:

  string    required text          string?   optional text
  text      required long text     url       required HTTPS address
  slug      unique id (a-z, 0-9, -)
  bool      yes or no              int       whole number

--root is the alerts-crud crate (default: crates/alerts-crud).";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    String,
    OptString,
    Text,
    Url,
    Slug,
    Bool,
    Int,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "string" => Kind::String,
            "string?" => Kind::OptString,
            "text" => Kind::Text,
            "url" => Kind::Url,
            "slug" => Kind::Slug,
            "bool" => Kind::Bool,
            "int" => Kind::Int,
            _ => return None,
        })
    }

    fn rust(self) -> &'static str {
        match self {
            Kind::OptString => "Option<String>",
            Kind::Bool => "bool",
            Kind::Int => "i64",
            Kind::String | Kind::Text | Kind::Url | Kind::Slug => "String",
        }
    }

    fn sql(self) -> &'static str {
        match self {
            Kind::OptString => "TEXT",
            Kind::Bool | Kind::Int => "INTEGER NOT NULL",
            Kind::Slug => "TEXT NOT NULL UNIQUE",
            Kind::String | Kind::Text | Kind::Url => "TEXT NOT NULL",
        }
    }

    fn field_kind(self) -> &'static str {
        match self {
            Kind::Text => "LongText",
            Kind::Url => "Url",
            Kind::Bool => "Bool",
            Kind::Int => "Int",
            Kind::String | Kind::OptString | Kind::Slug => "Text",
        }
    }

    /// The `Checks` call for a field of this kind, if it needs one.
    fn check(self, f: &str) -> Option<String> {
        match self {
            Kind::String | Kind::Text => Some(format!(".required(\"{f}\", &self.{f})")),
            Kind::Slug => Some(format!(".slug(\"{f}\", &self.{f})")),
            Kind::Url => Some(format!(
                ".check(\"{f}\", crate::validate::is_https_url(&self.{f}), \"Use um endereço HTTPS.\")"
            )),
            Kind::OptString | Kind::Bool | Kind::Int => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    name: String,
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Scaffold {
    /// The table and resource name, plural snake_case.
    name: String,
    fields: Vec<Field>,
}

fn is_ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

const RESERVED: &[&str] = &[
    "id",
    "created_at",
    "updated_at",
    "type",
    "struct",
    "fn",
    "mod",
    "self",
    "crate",
];

fn parse_args(args: &[String]) -> Result<(Scaffold, PathBuf), String> {
    let mut root = PathBuf::from("crates/alerts-crud");
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--root" {
            root = it.next().ok_or("--root needs a directory")?.into();
        } else {
            rest.push(a.as_str());
        }
    }
    let (name, specs) = rest.split_first().ok_or("no resource name")?;
    if !is_ident(name) || !name.ends_with('s') {
        return Err(format!(
            "{name}: use a plural snake_case name, for example rivers"
        ));
    }
    if specs.is_empty() {
        return Err("no fields".into());
    }
    let mut fields: Vec<Field> = Vec::new();
    for spec in specs {
        let (f, k) = spec
            .split_once(':')
            .ok_or_else(|| format!("{spec}: use <name>:<kind>"))?;
        if !is_ident(f) || RESERVED.contains(&f) {
            return Err(format!("{f}: not a free snake_case field name"));
        }
        if fields.iter().any(|x| x.name == f) {
            return Err(format!("{f}: the field is repeated"));
        }
        let kind = Kind::parse(k).ok_or_else(|| format!("{k}: unknown kind"))?;
        fields.push(Field {
            name: f.into(),
            kind,
        });
    }
    Ok((
        Scaffold {
            name: (*name).into(),
            fields,
        },
        root,
    ))
}

impl Scaffold {
    /// `rivers` is `River`, `water_levels` is `WaterLevel`.
    fn struct_name(&self) -> String {
        let singular = self.name.strip_suffix('s').unwrap_or(&self.name);
        singular
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn label(&self) -> String {
        let mut c = self.name.chars();
        c.next()
            .map(|f| f.to_ascii_uppercase().to_string() + &c.as_str().replace('_', " "))
            .unwrap_or_default()
    }

    fn model(&self) -> String {
        let s = self.struct_name();
        let mut fields = String::new();
        let mut checks = String::new();
        let mut specs = String::new();
        for f in &self.fields {
            let _ = writeln!(fields, "        {}: {},", f.name, f.kind.rust());
            if let Some(c) = f.kind.check(&f.name) {
                let _ = write!(checks, "\n            {c}");
            }
            let _ = writeln!(
                specs,
                "        field(\"{n}\", \"{n}\", FieldKind::{k}, \"\", true),",
                n = f.name,
                k = f.kind.field_kind()
            );
        }
        format!(
            r#"//! {label}: written by `just generate scaffold`. Write the pt-BR labels of `SPEC` and the
//! checks of `validate`.

use crate::model;
use crate::schema::{{FieldKind, ModuleSpec, Op, field}};
use crate::validate::{{Checks, FieldError, Validate}};

model! {{
    /// One record of {name}.
    {s}, {s}Params in "{name}" {{
{fields}    }}
}}

impl Validate for {s}Params {{
    fn validate(&self) -> Result<(), Vec<FieldError>> {{
        Checks::new(){checks}
            .finish()
    }}
}}

pub const SPEC: ModuleSpec = ModuleSpec {{
    resource: "{name}",
    label: "{label}",
    singular: "{s}",
    about: "",
    fields: &[
{specs}    ],
    ops: Op::CRUD,
    actions: &[],
}};
"#,
            label = self.label(),
            name = self.name,
        )
    }

    fn controller(&self) -> String {
        format!(
            r#"//! Controller of {name}: the default CRUD.

use super::Resource;
use crate::models::{name}::{{SPEC, {s}}};

pub fn resource() -> Resource {{
    Resource::crud::<{s}>(&SPEC)
}}
"#,
            name = self.name,
            s = self.struct_name()
        )
    }

    fn migration(&self, id: &str) -> String {
        let mut cols = String::new();
        for f in &self.fields {
            let _ = writeln!(cols, "        {} {},", f.name, f.kind.sql());
        }
        format!(
            r#"pub const MIGRATION: (&str, &str) = (
    "{id}",
    "CREATE TABLE {name} (
        id INTEGER PRIMARY KEY,
{cols}        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    )",
);
"#,
            name = self.name
        )
    }
}

/// Puts `line` before the line that holds `marker`, with the marker's indentation.
fn insert_before(src: &str, marker: &str, line: &str) -> Result<String, String> {
    let at = src
        .find(marker)
        .ok_or_else(|| format!("marker not found: {marker}"))?;
    let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
    let indent = &src[line_start..at];
    Ok(format!(
        "{}{indent}{line}\n{}",
        &src[..line_start],
        &src[line_start..]
    ))
}

fn edit(path: &Path, marker: &str, line: &str) -> Result<(), Box<dyn Error>> {
    let src = fs::read_to_string(path)?;
    fs::write(path, insert_before(&src, marker, line)?)?;
    Ok(())
}

/// Today as `YYYYMMDD` (UTC), from days since 1970-01-01 (Howard Hinnant's algorithm).
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}")
}

fn write(sc: &Scaffold, root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let src = root.join("src");
    let migrations = src.join("migrations");
    let seq = fs::read_dir(&migrations)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name();
            let n = n.to_string_lossy();
            n.starts_with('m') && n[1..].starts_with(|c: char| c.is_ascii_digit())
        })
        .count()
        + 1;
    let mig = format!("m{}_{seq:06}_create_{}", today(), sc.name);
    let files = [
        (src.join(format!("models/{}.rs", sc.name)), sc.model()),
        (
            src.join(format!("controllers/{}.rs", sc.name)),
            sc.controller(),
        ),
        (migrations.join(format!("{mig}.rs")), sc.migration(&mig)),
    ];
    if let Some((p, _)) = files.iter().find(|(p, _)| p.exists()) {
        return Err(format!("{} exists: the resource is already there", p.display()).into());
    }
    if fs::read_to_string(src.join("models/mod.rs"))?.contains(&format!("pub mod {};", sc.name)) {
        return Err(format!("{}: the resource is already registered", sc.name).into());
    }
    for (p, body) in &files {
        fs::write(p, body)?;
    }
    let n = &sc.name;
    edit(
        &src.join("models/mod.rs"),
        "// scaffold: models",
        &format!("pub mod {n};"),
    )?;
    edit(
        &src.join("controllers/mod.rs"),
        "// scaffold: controllers",
        &format!("pub mod {n};"),
    )?;
    edit(
        &src.join("controllers/mod.rs"),
        "// scaffold: resources",
        &format!("{n}::resource(),"),
    )?;
    edit(
        &migrations.join("mod.rs"),
        "// scaffold: migration modules",
        &format!("mod {mig};"),
    )?;
    edit(
        &migrations.join("mod.rs"),
        "// scaffold: migrations",
        &format!("{mig}::MIGRATION,"),
    )?;
    Ok(files.into_iter().map(|(p, _)| p).collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let (sc, root) = parse_args(&args).map_err(|e| format!("{e}\n\n{USAGE}"))?;
    for p in write(&sc, &root)? {
        println!("scaffold: wrote {}", p.display());
    }
    println!(
        "scaffold: registered {} in models, controllers and migrations. Write the pt-BR labels in SPEC.",
        sc.name
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_fields() {
        let (sc, root) =
            parse_args(&args("water_levels gauge:slug level:int note:string?")).unwrap();
        assert_eq!(root, PathBuf::from("crates/alerts-crud"));
        assert_eq!(sc.struct_name(), "WaterLevel");
        assert_eq!(sc.fields[2].kind, Kind::OptString);
        assert!(parse_args(&args("river name:string")).is_err());
        assert!(parse_args(&args("rivers id:int")).is_err());
        assert!(parse_args(&args("rivers name:float")).is_err());
        assert!(parse_args(&args("rivers a:int a:int")).is_err());
    }

    #[test]
    fn renders_the_three_files() {
        let (sc, _) = parse_args(&args("rivers slug:slug name:string site:url")).unwrap();
        let m = sc.model();
        assert!(m.contains("River, RiverParams in \"rivers\""));
        assert!(m.contains(".slug(\"slug\", &self.slug)"));
        assert!(m.contains("is_https_url(&self.site)"));
        assert!(sc.controller().contains("Resource::crud::<River>(&SPEC)"));
        let mig = sc.migration("m20261027_000005_create_rivers");
        assert!(mig.contains("slug TEXT NOT NULL UNIQUE,"));
    }

    #[test]
    fn inserts_before_the_marker() {
        let src = "vec![\n    a(),\n    // scaffold: resources\n]\n";
        let out = insert_before(src, "// scaffold: resources", "b(),").unwrap();
        assert_eq!(
            out,
            "vec![\n    a(),\n    b(),\n    // scaffold: resources\n]\n"
        );
        assert!(insert_before(src, "// nope", "b()").is_err());
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 8);
        assert!(t.starts_with("20"));
    }
}
