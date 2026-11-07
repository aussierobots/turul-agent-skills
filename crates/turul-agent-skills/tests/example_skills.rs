//! The workspace's example and reference skills are valid and draw no authoring advisories.
//! They live outside the library package, so this file is not published.

use std::fs;
use std::path::{Path, PathBuf};

use turul_agent_skills::{advise, load, validate};

fn examples_skills() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("examples/skill-llm/skills")
}

#[test]
fn example_and_reference_skills_have_no_advisories() {
    let root = examples_skills();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for top in [root.clone(), root.join("reference")] {
        for entry in fs::read_dir(top).unwrap() {
            let path = entry.unwrap().path();
            if path.join("SKILL.md").is_file() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    assert!(dirs.len() >= 6, "{dirs:?}");
    for dir in dirs {
        let skill = load(&dir).unwrap().skill;
        assert!(
            advise(&skill).is_empty(),
            "{}: {:?}",
            dir.display(),
            advise(&skill)
        );
    }
}

#[test]
fn skill_review_passes_validate_and_advise() {
    let dir = examples_skills().join("skill-review");
    assert!(validate(&dir).is_empty());
    let skill = load(&dir).unwrap().skill;
    assert!(advise(&skill).is_empty());
    assert_eq!(
        skill.properties.license.as_deref(),
        Some("MIT OR Apache-2.0")
    );
    for file in ["references/best-practices.md", "assets/report-template.md"] {
        assert!(dir.join(file).is_file(), "{file}");
        assert!(
            skill.body.contains(file),
            "body must say when to use {file}"
        );
    }
}
