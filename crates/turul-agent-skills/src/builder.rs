use std::collections::BTreeMap;

use crate::name::{NameError, SkillName};
use crate::parse::SkillProperties;
use crate::skill::Skill;

/// Builds a new [`Skill`] from its fields; the one way to construct a skill that
/// was not loaded from disk.
///
/// `build` parses the name through [`SkillName::parse`]. Length and directory
/// rules are checked by [`validate`](crate::validate), not here.
///
/// ```
/// use turul_agent_skills::SkillBuilder;
///
/// let skill = SkillBuilder::new("summarize", "Summarize a document.")
///     .license("MIT")
///     .body("Answer in three bullets.\n")
///     .build()
///     .unwrap();
/// assert_eq!(skill.properties.name.as_str(), "summarize");
/// ```
#[derive(Clone, Debug)]
pub struct SkillBuilder {
    name: String,
    description: String,
    license: Option<String>,
    compatibility: Option<String>,
    metadata: BTreeMap<String, String>,
    allowed_tools: Option<String>,
    body: String,
}

impl SkillBuilder {
    /// A builder with the two required fields; the body is empty.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        SkillBuilder {
            name: name.into(),
            description: description.into(),
            license: None,
            compatibility: None,
            metadata: BTreeMap::new(),
            allowed_tools: None,
            body: String::new(),
        }
    }

    /// Sets `license`.
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Sets `compatibility`.
    pub fn compatibility(mut self, compatibility: impl Into<String>) -> Self {
        self.compatibility = Some(compatibility.into());
        self
    }

    /// Sets `allowed-tools`.
    pub fn allowed_tools(mut self, allowed_tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(allowed_tools.into());
        self
    }

    /// Adds one `metadata` entry; a repeated key keeps the last value.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Sets the Markdown body.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// The skill, or the reasons the name is not a legal [`SkillName`].
    pub fn build(self) -> Result<Skill, NameError> {
        let mut properties = SkillProperties::new(SkillName::parse(&self.name)?, self.description);
        properties.license = self.license;
        properties.compatibility = self.compatibility;
        properties.metadata = self.metadata;
        properties.allowed_tools = self.allowed_tools;
        Ok(Skill::new(properties, self.body))
    }
}
