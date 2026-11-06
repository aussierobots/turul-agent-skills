//! Authoring advisories report the two measurable specification recommendations:
//! a long `SKILL.md` and file references nested deeper than one level.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use turul_agent_skills::{ErrorCode, MAX_RECOMMENDED_LINES, Skill, advise, load, validate};

const FRONTMATTER: &str = "---\nname: demo\ndescription: Demo skill.\n---\n";

/// Writes `demo/SKILL.md` with `body` plus the given extra files, and loads it.
fn skill_with(body: &str, files: &[(&str, &str)]) -> (TempDir, Skill) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("{FRONTMATTER}{body}")).unwrap();
    for (path, text) in files {
        let file = dir.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    let skill = load(&dir).unwrap().skill;
    (tmp, skill)
}

fn codes(skill: &Skill) -> Vec<ErrorCode> {
    advise(skill).iter().map(|a| a.code()).collect()
}

/// A body that makes the rendered file exactly `total` lines.
fn body_for_total_lines(total: usize) -> String {
    let header = FRONTMATTER.lines().count();
    "line\n".repeat(total - header)
}

#[test]
fn clean_skill_has_no_advisories() {
    let (_tmp, skill) = skill_with("Say hello.\n", &[]);
    assert!(advise(&skill).is_empty());
}

#[test]
fn recommended_line_limit_is_500() {
    assert_eq!(MAX_RECOMMENDED_LINES, 500);
}

#[test]
fn skill_md_over_500_lines_is_advised() {
    let (_tmp, skill) = skill_with(&body_for_total_lines(501), &[]);
    assert_eq!(skill.render().lines().count(), 501);
    let advisories = advise(&skill);
    assert_eq!(codes(&skill), [ErrorCode::SkillMdTooLong]);
    assert_eq!(ErrorCode::SkillMdTooLong.as_str(), "skill-md-too-long");
    assert!(advisories[0].message().contains("501"));
    assert!(advisories[0].message().contains("500"));
}

#[test]
fn skill_md_of_exactly_500_lines_is_not_advised() {
    let (_tmp, skill) = skill_with(&body_for_total_lines(500), &[]);
    assert_eq!(skill.render().lines().count(), 500);
    assert!(advise(&skill).is_empty());
}

#[test]
fn advisories_do_not_fail_validate_or_load() {
    let (tmp, skill) = skill_with(&body_for_total_lines(501), &[]);
    assert!(!advise(&skill).is_empty());
    assert!(validate(&tmp.path().join("demo")).is_empty());
    assert!(
        load(&tmp.path().join("demo"))
            .unwrap()
            .diagnostics
            .is_empty()
    );
}

#[test]
fn one_level_references_are_not_advised() {
    let (_tmp, skill) = skill_with(
        "Read [the guide](references/a.md) and `scripts/run.py`.\n",
        &[
            ("references/a.md", "Nothing else here.\n"),
            ("scripts/run.py", "print('x')\n"),
        ],
    );
    assert!(advise(&skill).is_empty());
}

#[test]
fn reference_chain_is_advised_naming_both_files() {
    let (_tmp, skill) = skill_with(
        "See [a](references/a.md).\n",
        &[
            ("references/a.md", "Continue in `references/b.md`.\n"),
            ("references/b.md", "End.\n"),
        ],
    );
    let advisories = advise(&skill);
    assert_eq!(codes(&skill), [ErrorCode::NestedReference]);
    assert_eq!(ErrorCode::NestedReference.as_str(), "nested-reference");
    let message = advisories[0].message();
    assert!(message.contains("references/a.md"), "{message}");
    assert!(message.contains("references/b.md"), "{message}");
}

#[test]
fn links_relative_to_the_referencing_file_count() {
    let (_tmp, skill) = skill_with(
        "See `references/a.md`.\n",
        &[
            ("references/a.md", "Continue in [b](b.md).\n"),
            ("references/b.md", "End.\n"),
        ],
    );
    assert_eq!(codes(&skill), [ErrorCode::NestedReference]);
}

#[test]
fn urls_anchors_absolute_and_escaping_targets_are_ignored() {
    let (_tmp, skill) = skill_with(
        "[web](https://example.com/x.md) [top](#usage) [abs](/etc/hosts) \
         [up](../outside.md) `../outside.md` [mail](mailto:a@b.c)\n",
        &[],
    );
    assert!(advise(&skill).is_empty());

    // A second-level target that escapes the skill directory is not a chain either.
    let (tmp, skill) = skill_with(
        "See [a](references/a.md).\n",
        &[(
            "references/a.md",
            "See [out](../../outside.md) and https://x.test/y.md.\n",
        )],
    );
    fs::write(tmp.path().join("outside.md"), "x").unwrap();
    assert!(advise(&skill).is_empty());
}

#[test]
fn references_that_do_not_exist_are_ignored() {
    let (_tmp, skill) = skill_with("See `references/missing.md`.\n", &[]);
    assert!(advise(&skill).is_empty());
}

#[test]
fn references_inside_code_fences_are_not_followed() {
    let (_tmp, skill) = skill_with(
        "```\n[a](references/a.md)\n```\n",
        &[
            ("references/a.md", "See `references/b.md`.\n"),
            ("references/b.md", "End.\n"),
        ],
    );
    assert!(advise(&skill).is_empty());
}

#[cfg(unix)]
#[test]
fn symlinks_leaving_the_skill_directory_are_not_read() {
    let (tmp, skill) = skill_with("See [a](references/a.md).\n", &[]);
    let outside = tmp.path().join("outside.md");
    fs::write(&outside, "See `references/b.md`.\n").unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(dir.join("references/b.md"), "End.\n").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("references/a.md")).unwrap();
    assert!(advise(&skill).is_empty());
}

#[test]
fn skill_without_a_location_gets_only_the_line_check() {
    let short = Skill::builder("demo", "Demo skill.")
        .body("See [a](references/a.md).\n")
        .build()
        .unwrap();
    assert!(advise(&short).is_empty());

    let long = Skill::builder("demo", "Demo skill.")
        .body("line\n".repeat(600))
        .build()
        .unwrap();
    assert_eq!(
        advise(&long).iter().map(|a| a.code()).collect::<Vec<_>>(),
        [ErrorCode::SkillMdTooLong]
    );
}

#[test]
fn advisories_are_deterministic_and_ordered() {
    let mut body = body_for_total_lines(501);
    body.push_str("See [b](references/b.md) and [a](references/a.md).\n");
    let (_tmp, skill) = skill_with(
        &body,
        &[
            ("references/a.md", "Next `references/c.md`.\n"),
            ("references/b.md", "Next `references/c.md`.\n"),
            ("references/c.md", "End.\n"),
        ],
    );
    let first = advise(&skill);
    assert_eq!(first, advise(&skill));
    let order: Vec<ErrorCode> = first.iter().map(|a| a.code()).collect();
    assert_eq!(
        order,
        [
            ErrorCode::SkillMdTooLong,
            ErrorCode::NestedReference,
            ErrorCode::NestedReference
        ]
    );
    // Body order: b before a.
    assert!(first[1].message().contains("references/b.md"));
    assert!(first[2].message().contains("references/a.md"));
}

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
