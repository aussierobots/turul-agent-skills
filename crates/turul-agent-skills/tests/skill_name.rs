//! Identity rules of `SkillName`: one constructor, trim then NFKC, five violations.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use turul_agent_skills::{NameViolation, SkillName};

fn violations(input: &str) -> Vec<NameViolation> {
    SkillName::parse(input)
        .expect_err("must be rejected")
        .violations
}

fn hash_of(name: &SkillName) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn parse_rejects_empty_and_whitespace() {
    assert_eq!(violations(""), vec![NameViolation::Empty]);
    assert_eq!(violations("  \t\n"), vec![NameViolation::Empty]);
    assert_eq!(SkillName::parse("  ").unwrap_err().normalized, "");
}

#[test]
fn parse_rejects_uppercase() {
    assert_eq!(violations("MySkill"), vec![NameViolation::NotLowercase]);
    assert_eq!(violations("НАВЫК"), vec![NameViolation::NotLowercase]);
}

#[test]
fn parse_rejects_leading_or_trailing_hyphen() {
    assert_eq!(violations("-my-skill"), vec![NameViolation::HyphenEdge]);
    assert_eq!(violations("my-skill-"), vec![NameViolation::HyphenEdge]);
}

#[test]
fn parse_rejects_consecutive_hyphens() {
    assert_eq!(
        violations("my--skill"),
        vec![NameViolation::ConsecutiveHyphens]
    );
}

#[test]
fn parse_rejects_invalid_chars() {
    assert_eq!(violations("my_skill"), vec![NameViolation::InvalidChars]);
    assert_eq!(violations("my skill"), vec![NameViolation::InvalidChars]);
    // Devanagari vowel signs and virama are marks, which Python's isalnum rejects.
    assert_eq!(violations("हिन्दी"), vec![NameViolation::InvalidChars]);
}

#[test]
fn parse_reports_every_failing_rule_in_order() {
    assert_eq!(
        violations("-My--Skill_"),
        vec![
            NameViolation::NotLowercase,
            NameViolation::HyphenEdge,
            NameViolation::ConsecutiveHyphens,
            NameViolation::InvalidChars,
        ]
    );
}

#[test]
fn parse_accepts_unicode_lowercase() {
    for input in ["技能", "мой-навык", "навык", "my-skill-2", "a"] {
        let name = SkillName::parse(input).unwrap();
        assert_eq!(name.as_str(), input);
    }
}

#[test]
fn parse_trims_then_nfkc_normalizes() {
    let decomposed = SkillName::parse("  cafe\u{301}\n").unwrap();
    assert_eq!(decomposed.as_str(), "caf\u{e9}");
    assert_eq!(decomposed, SkillName::parse("caf\u{e9}").unwrap());
    assert_eq!(
        hash_of(&decomposed),
        hash_of(&SkillName::parse("caf\u{e9}").unwrap())
    );
    // Fullwidth letters fold to ASCII under NFKC.
    assert_eq!(SkillName::parse("ｓｋｉｌｌ").unwrap().as_str(), "skill");
}

#[test]
fn parse_accepts_overlong() {
    let long = "a".repeat(70);
    assert_eq!(SkillName::parse(&long).unwrap().as_str(), long);
}

#[test]
fn normalized_form_is_exposed_on_error() {
    let error = SkillName::parse("  My_Skill ").unwrap_err();
    assert_eq!(error.normalized, "My_Skill");
}

#[test]
fn distinct_inputs_distinct_names() {
    let inputs = ["a", "b", "ab", "a-b", "技能", "навык", "caf\u{e9}", "cafe"];
    let names: HashSet<SkillName> = inputs
        .iter()
        .map(|i| SkillName::parse(i).unwrap())
        .collect();
    assert_eq!(names.len(), inputs.len());
}

#[test]
fn display_round_trips() {
    let name = SkillName::parse("my-skill").unwrap();
    assert_eq!(name.to_string(), name.as_str());
    assert_eq!(SkillName::parse(&name.to_string()).unwrap(), name);
}

#[test]
fn ord_is_byte_order() {
    let mut names: Vec<SkillName> = ["b", "a-b", "a", "é", "z"]
        .iter()
        .map(|i| SkillName::parse(i).unwrap())
        .collect();
    names.sort();
    let order: Vec<&str> = names.iter().map(SkillName::as_str).collect();
    assert_eq!(order, ["a", "a-b", "b", "z", "é"]);
}
