//! Each identity and artifact has one owning file. Scans the sources of both
//! crates and the examples; tests are exempt.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file under each crate's `src/` and under `examples/`.
fn sources() -> Vec<(PathBuf, String)> {
    let root = workspace_root();
    let mut roots = vec![root.join("examples")];
    for krate in fs::read_dir(root.join("crates")).unwrap() {
        roots.push(krate.unwrap().path().join("src"));
    }
    let mut files = Vec::new();
    for dir in roots {
        collect(&dir, &mut files);
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect()
}

/// Files that contain `needle` and are not an allowed owner, by file name.
fn offenders(needle: &str, owners: &[&str]) -> Vec<String> {
    sources()
        .into_iter()
        .filter(|(path, text)| {
            text.contains(needle)
                && !owners.iter().any(|owner| {
                    path.ends_with(Path::new("crates/turul-agent-skills/src").join(owner))
                })
        })
        .map(|(path, _)| path.display().to_string())
        .collect()
}

fn owned_source(file: &str) -> String {
    fs::read_to_string(
        workspace_root()
            .join("crates/turul-agent-skills/src")
            .join(file),
    )
    .unwrap()
}

#[test]
fn skill_name_constructed_only_in_name_rs() {
    assert_eq!(offenders("SkillName(", &["name.rs"]), Vec::<String>::new());
}

#[test]
fn skill_properties_literal_only_in_parse_rs() {
    let found: Vec<String> = sources()
        .into_iter()
        .filter(|(path, text)| {
            !path.ends_with("crates/turul-agent-skills/src/parse.rs")
                && text.lines().any(|line| {
                    line.contains("SkillProperties {") && !line.trim_start().starts_with("//")
                })
        })
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn skill_properties_fields_are_spec_fields() {
    let text = owned_source("parse.rs");
    let body = text
        .split("pub struct SkillProperties {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("SkillProperties is declared in parse.rs");
    let fields: Vec<&str> = body
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub "))
        .filter_map(|line| line.split(':').next())
        .collect();
    assert_eq!(
        fields,
        [
            "name",
            "description",
            "license",
            "compatibility",
            "metadata",
            "allowed_tools"
        ]
    );
}

#[test]
fn parse_skill_md_called_only_from_validate_and_load() {
    let found: Vec<String> = sources()
        .into_iter()
        .filter(|(path, text)| {
            let allowed = ["validate.rs", "load.rs", "parse.rs"]
                .iter()
                .any(|f| path.ends_with(Path::new("crates/turul-agent-skills/src").join(f)));
            !allowed
                && text.lines().any(|line| {
                    line.contains("parse_skill_md(") && !line.contains("fn parse_skill_md(")
                })
        })
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn discover_calls_load_only() {
    let text = owned_source("discover.rs");
    assert!(text.contains("load("), "discover.rs must call load");
    for other in [
        "parse_skill_md(",
        "read_and_parse(",
        "check(",
        "validate(",
        "Skill::new(",
        "SkillName::parse(",
        "SkillProperties::new(",
    ] {
        assert!(!text.contains(other), "discover.rs calls {other}");
    }
}
