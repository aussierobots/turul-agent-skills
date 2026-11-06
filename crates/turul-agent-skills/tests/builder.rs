//! `SkillBuilder` is the one way to construct a new skill.

use std::collections::BTreeMap;

use tempfile::TempDir;
use turul_agent_skills::{BuildError, NameViolation, Skill, SkillBuilder, load, validate};

#[test]
fn minimal_build_has_required_fields_and_empty_rest() {
    let skill = SkillBuilder::new("my-skill", "Does a thing.")
        .build()
        .unwrap();
    assert_eq!(skill.properties.name.as_str(), "my-skill");
    assert_eq!(skill.properties.description, "Does a thing.");
    assert_eq!(skill.properties.license, None);
    assert_eq!(skill.properties.compatibility, None);
    assert_eq!(skill.properties.allowed_tools, None);
    assert!(skill.properties.metadata.is_empty());
    assert_eq!(skill.body, "");
    assert!(skill.extensions().is_empty());
    assert!(skill.location().is_none());
}

#[test]
fn every_setter_lands_in_the_skill() {
    let skill = Skill::builder("my-skill", "d")
        .license("MIT")
        .compatibility("Needs a shell")
        .allowed_tools("Bash(git:*) Read")
        .metadata("author", "me")
        .body("Do it.\n")
        .build()
        .unwrap();
    assert_eq!(skill.properties.license.as_deref(), Some("MIT"));
    assert_eq!(
        skill.properties.compatibility.as_deref(),
        Some("Needs a shell")
    );
    assert_eq!(
        skill.properties.allowed_tools.as_deref(),
        Some("Bash(git:*) Read")
    );
    assert_eq!(skill.properties.metadata["author"], "me");
    assert_eq!(skill.body, "Do it.\n");
}

#[test]
fn repeated_metadata_accumulates() {
    let skill = SkillBuilder::new("my-skill", "d")
        .metadata("b", "2")
        .metadata("a", "1")
        .metadata("a", "3")
        .build()
        .unwrap();
    let expected: BTreeMap<String, String> = [("a", "3"), ("b", "2")]
        .map(|(k, v)| (k.into(), v.into()))
        .into();
    assert_eq!(skill.properties.metadata, expected);
}

#[test]
fn build_normalizes_the_name_through_skill_name() {
    let skill = SkillBuilder::new(" ｇreet ", "d").build().unwrap();
    assert_eq!(skill.properties.name.as_str(), "greet");
}

#[test]
fn invalid_name_is_a_name_error_with_violations() {
    let Err(BuildError::Name(error)) = SkillBuilder::new("-Bad", "d").build() else {
        panic!("expected a name error");
    };
    assert_eq!(
        error.violations,
        vec![NameViolation::NotLowercase, NameViolation::HyphenEdge]
    );
}

#[test]
fn built_skill_writes_validates_and_reloads_equal() {
    let parent = TempDir::new().unwrap();
    let skill = SkillBuilder::new("my-skill", "d")
        .license("MIT")
        .metadata("k", "v")
        .allowed_tools("Read")
        .compatibility("c")
        .body("Body\n")
        .build()
        .unwrap();
    let file = skill.write_to(parent.path()).unwrap();
    let dir = file.parent().unwrap();
    assert!(validate(dir).is_empty());
    assert_eq!(load(dir).unwrap().skill, skill);
}

#[test]
fn blank_description_is_rejected_with_the_validation_message() {
    for blank in ["", "  \t\n"] {
        let error = SkillBuilder::new("my-skill", blank).build().unwrap_err();
        assert!(matches!(error, BuildError::DescriptionEmpty));
        assert_eq!(
            error.to_string(),
            "Field 'description' must be a non-empty string"
        );
    }
}

#[test]
fn padded_description_is_trimmed_and_round_trips() {
    let skill = SkillBuilder::new("my-skill", "  Does a thing.\n")
        .build()
        .unwrap();
    assert_eq!(skill.properties.description, "Does a thing.");
    let parent = TempDir::new().unwrap();
    let file = skill.write_to(parent.path()).unwrap();
    let dir = file.parent().unwrap();
    assert!(validate(dir).is_empty());
    assert_eq!(load(dir).unwrap().skill, skill);
}
