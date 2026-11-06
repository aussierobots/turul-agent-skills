//! Fixture access shared by the integration tests.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("crates/turul-agent-skills/tests/fixtures")
}

pub fn fixture_ids() -> Vec<String> {
    let mut ids: Vec<String> = fs::read_dir(fixtures_dir())
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

pub fn expected(id: &str) -> Value {
    let text = fs::read_to_string(fixtures_dir().join(id).join("expected.json")).unwrap();
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{id}: expected.json: {e}"))
}

/// Child directories in byte order of their names.
pub fn subdirectories(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort_by(|a, b| {
        a.file_name()
            .unwrap()
            .as_encoded_bytes()
            .cmp(b.file_name().unwrap().as_encoded_bytes())
    });
    dirs
}

/// The directory a case validates and loads, built in `tmp` when git cannot hold it.
pub fn case_path(id: &str, expected: &Value, tmp: &TempDir) -> PathBuf {
    let root = fixtures_dir().join(id);
    match expected.get("layout").and_then(Value::as_str) {
        None => {
            let dirs = subdirectories(&root);
            assert_eq!(dirs.len(), 1, "{id}: exactly one case directory");
            dirs.into_iter().next().unwrap()
        }
        Some("empty-dir") => {
            let path = tmp.path().join("my-skill");
            fs::create_dir(&path).unwrap();
            path
        }
        Some("missing") => tmp.path().join("nonexistent"),
        Some("regular-file") => {
            let path = tmp.path().join("file.txt");
            fs::write(&path, "test").unwrap();
            path
        }
        Some("both-cases") => {
            let path = tmp.path().join("my-skill");
            fs::create_dir(&path).unwrap();
            fs::copy(root.join("my-skill/SKILL.md"), path.join("SKILL.md")).unwrap();
            fs::copy(root.join("lowercase.md"), path.join("skill.md")).unwrap();
            path
        }
        Some(other) => panic!("{id}: unknown layout {other}"),
    }
}
