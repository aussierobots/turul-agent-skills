use std::path::{Path, PathBuf};

use crate::builder::SkillBuilder;
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
    /// A builder for a new skill; see [`SkillBuilder`].
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> SkillBuilder {
        SkillBuilder::new(name, description)
    }

    /// A skill with no extensions and no location.
    pub(crate) fn new(properties: SkillProperties, body: impl Into<String>) -> Skill {
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
    /// use turul_agent_skills::Skill;
    /// let mut skill = Skill::builder("a", "d").build().unwrap();
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
