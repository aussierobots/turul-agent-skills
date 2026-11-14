//! Publish crates must stay free of LLM, A2A, async-runtime and HTTP
//! dependencies. LLM clients are used only by `examples/`; A2A projection
//! lives in turul-a2a on top of this crate's types.

use std::path::{Path, PathBuf};

const FORBIDDEN_PREFIXES: &[&str] = &["turul-llm", "turul-a2a", "turul-mcp", "tokio", "reqwest"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives at <root>/crates/<name>")
        .to_path_buf()
}

fn publish_crate_manifests() -> Vec<PathBuf> {
    let crates_dir = workspace_root().join("crates");
    let mut manifests: Vec<PathBuf> = std::fs::read_dir(&crates_dir)
        .expect("crates/ is readable")
        .map(|entry| entry.expect("dir entry").path().join("Cargo.toml"))
        .filter(|manifest| manifest.is_file())
        .collect();
    manifests.sort();
    manifests
}

/// Names of every normal dependency, including target-specific tables.
fn normal_dependency_names(manifest: &toml::Table) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Some(deps) = manifest.get("dependencies").and_then(|v| v.as_table()) {
        names.extend(deps.keys().cloned());
    }
    if let Some(targets) = manifest.get("target").and_then(|v| v.as_table()) {
        for target in targets.values() {
            if let Some(deps) = target.get("dependencies").and_then(|v| v.as_table()) {
                names.extend(deps.keys().cloned());
            }
        }
    }
    names
}

fn forbidden_in(manifest_text: &str) -> Vec<String> {
    let manifest: toml::Table = manifest_text.parse().expect("valid Cargo.toml");
    normal_dependency_names(&manifest)
        .into_iter()
        .filter(|name| FORBIDDEN_PREFIXES.iter().any(|p| name.starts_with(p)))
        .collect()
}

#[test]
fn publish_crates_have_no_forbidden_dependencies() {
    let manifests = publish_crate_manifests();
    assert!(
        manifests.len() >= 2,
        "expected both publish crates, found {manifests:?}"
    );
    for path in manifests {
        let text = std::fs::read_to_string(&path).expect("manifest readable");
        let forbidden = forbidden_in(&text);
        assert!(
            forbidden.is_empty(),
            "{} depends on {forbidden:?}",
            path.display()
        );
    }
}

#[test]
fn detector_catches_plain_and_target_specific_dependencies() {
    let manifest = r#"
        [dependencies]
        turul-llm-core = "0.1"
        serde = "1"
        [target.'cfg(unix)'.dependencies]
        tokio = "1"
        [dev-dependencies]
        reqwest = "0.13"
    "#;
    assert_eq!(forbidden_in(manifest), vec!["turul-llm-core", "tokio"]);
}
