//! The crate stays free of A2A concepts; A2A skills are plain spec skills with
//! namespaced metadata.

use std::fs;
use std::path::{Path, PathBuf};

use turul_agent_skills::{load, validate};

mod common;
use common::fixtures_dir;

fn migrated_dir() -> PathBuf {
    fixtures_dir().join("house-a2a-migrated/greet")
}

#[test]
fn migrated_a2a_fixture_is_strict_valid() {
    assert_eq!(validate(&migrated_dir()), []);
}

#[test]
fn migrated_a2a_fixture_loads_clean_and_renders_identically() {
    let loaded = load(&migrated_dir()).expect("loads");
    assert!(loaded.diagnostics.is_empty());
    let file = fs::read_to_string(migrated_dir().join("SKILL.md")).unwrap();
    assert_eq!(loaded.skill.render(), file);
}

#[test]
fn migrated_a2a_fixture_metadata_keys_are_namespaced() {
    let loaded = load(&migrated_dir()).expect("loads");
    let metadata = &loaded.skill.properties.metadata;
    assert!(!metadata.is_empty());
    for key in metadata.keys() {
        assert!(key.starts_with("a2a-"), "{key} is not namespaced");
    }
}

#[test]
fn migrated_a2a_fixture_referenced_assets_exist() {
    let loaded = load(&migrated_dir()).expect("loads");
    let paths: Vec<&String> = loaded
        .skill
        .properties
        .metadata
        .values()
        .filter(|value| value.starts_with("assets/"))
        .collect();
    assert!(!paths.is_empty());
    for path in paths {
        assert!(migrated_dir().join(path).is_file(), "{path} is missing");
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn public_api_has_no_a2a_symbol() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty());
    for file in files {
        let text = fs::read_to_string(&file).unwrap().to_lowercase();
        assert!(!text.contains("a2a"), "{} mentions a2a", file.display());
    }
}
