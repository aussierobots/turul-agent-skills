use std::path::{Path, PathBuf};

use crate::parse::{RawValue, SkillProperties};

/// A loaded or constructed skill: spec fields, extension keys and the body.
///
/// Equality compares `properties`, extensions and `body`; `location` is
/// provenance, not content.
#[derive(Clone, Debug)]
pub struct Skill {
    /// The specification fields.
    pub properties: SkillProperties,
    pub(crate) extensions: Vec<(String, RawValue)>,
    /// The Markdown after the frontmatter, verbatim.
    pub body: String,
    pub(crate) location: Option<PathBuf>,
}

impl Skill {
    /// A skill with no extensions and no location.
    pub fn new(properties: SkillProperties, body: impl Into<String>) -> Skill {
        Skill {
            properties,
            extensions: Vec::new(),
            body: body.into(),
            location: None,
        }
    }

    /// Frontmatter keys outside the specification, in source order.
    ///
    /// Only `load` fills them; outside the crate they are read-only.
    ///
    /// ```compile_fail
    /// use turul_agent_skills::{Skill, SkillName, SkillProperties};
    /// let name = SkillName::parse("a").unwrap();
    /// let mut skill = Skill::new(SkillProperties::new(name, "d"), "");
    /// skill.extensions.push(("k".to_string(), turul_agent_skills::RawValue::Seq(vec![])));
    /// ```
    pub fn extensions(&self) -> &[(String, RawValue)] {
        &self.extensions
    }

    /// The absolute, canonical path of the `SKILL.md` this skill was loaded from.
    pub fn location(&self) -> Option<&Path> {
        self.location.as_deref()
    }
}

impl PartialEq for Skill {
    fn eq(&self, other: &Skill) -> bool {
        self.properties == other.properties
            && self.extensions == other.extensions
            && self.body == other.body
    }
}

impl Eq for Skill {}
