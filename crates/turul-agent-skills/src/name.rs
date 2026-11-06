use std::fmt;

use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

/// The identity of a skill: trimmed, NFKC-normalized, and checked against the
/// name rules of the Agent Skills specification.
///
/// `SkillName::parse` is the only constructor, so two spellings of one name
/// are always the same value.
///
/// ```
/// use turul_agent_skills::SkillName;
///
/// let name = SkillName::parse("  my-skill ").unwrap();
/// assert_eq!(name.as_str(), "my-skill");
/// assert!(SkillName::parse("My_Skill").is_err());
/// ```
///
/// The field is private; a name cannot be built from a raw string.
///
/// ```compile_fail
/// use turul_agent_skills::SkillName;
/// let _ = SkillName(String::new());
/// ```
///
/// ```compile_fail
/// use turul_agent_skills::SkillName;
/// let _ = SkillName::from(String::new());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillName(String);

/// One identity rule a candidate name broke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameViolation {
    /// Nothing is left after trimming.
    Empty,
    /// The name is not equal to its lowercase form.
    NotLowercase,
    /// The name starts or ends with a hyphen.
    HyphenEdge,
    /// The name contains two hyphens in a row.
    ConsecutiveHyphens,
    /// A character is neither a hyphen nor a letter or number.
    InvalidChars,
}

/// Why a candidate name is not a [`SkillName`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameError {
    /// The trimmed, NFKC-normalized candidate.
    pub normalized: String,
    /// Every rule that failed, in rule order.
    pub violations: Vec<NameViolation>,
}

impl SkillName {
    /// Trims, NFKC-normalizes and checks `input`.
    pub fn parse(input: &str) -> Result<SkillName, NameError> {
        let normalized: String = input.trim().nfkc().collect();
        let mut violations = Vec::new();
        if normalized.is_empty() {
            violations.push(NameViolation::Empty);
        } else {
            if normalized != normalized.to_lowercase() {
                violations.push(NameViolation::NotLowercase);
            }
            if normalized.starts_with('-') || normalized.ends_with('-') {
                violations.push(NameViolation::HyphenEdge);
            }
            if normalized.contains("--") {
                violations.push(NameViolation::ConsecutiveHyphens);
            }
            if !normalized.chars().all(is_name_char) {
                violations.push(NameViolation::InvalidChars);
            }
        }
        if violations.is_empty() {
            Ok(SkillName(normalized))
        } else {
            Err(NameError {
                normalized,
                violations,
            })
        }
    }

    /// The normalized name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `-`, or a letter or number by Unicode general category (Python `str.isalnum`).
fn is_name_char(c: char) -> bool {
    c == '-'
        || matches!(
            c.general_category_group(),
            GeneralCategoryGroup::Letter | GeneralCategoryGroup::Number
        )
}
