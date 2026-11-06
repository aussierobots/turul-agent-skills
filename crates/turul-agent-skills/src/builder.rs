use std::collections::BTreeMap;

use crate::name::{NameError, SkillName};
use crate::parse::SkillProperties;
use crate::skill::Skill;
use crate::validate::DESCRIPTION_EMPTY_MESSAGE;

/// Why [`SkillBuilder::build`] produced no skill.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{BuildError, SkillBuilder};
///
/// let error = SkillBuilder::new("pdf-tools", "  ").build().unwrap_err();
/// assert_eq!(error, BuildError::DescriptionEmpty);
///
/// let error = SkillBuilder::new("PDF Tools", "Work with PDFs.").build().unwrap_err();
/// assert!(matches!(error, BuildError::Name(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildError {
    /// The name is not a legal [`SkillName`].
    #[error(transparent)]
    Name(#[from] NameError),
    /// The description is empty after trimming.
    #[error("{}", DESCRIPTION_EMPTY_MESSAGE)]
    DescriptionEmpty,
}

/// Builds a new [`Skill`] from its fields; the one way to construct a skill that
/// was not loaded from disk.
///
/// `build` parses the name through [`SkillName::parse`] and trims the
/// description, rejecting an empty one. The 1024-character description limit,
/// the other length limits and the directory rule are checked by
/// [`validate`](crate::validate), not here.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::SkillBuilder;
///
/// let skill = SkillBuilder::new("summarize", "Summarize a document.")
///     .license("MIT")
///     .compatibility("Needs no network access")
///     .allowed_tools("Read")
///     .metadata("author", "docs-team")
///     .body("Answer in three bullets.\n")
///     .build()?;
/// assert_eq!(skill.properties.name.as_str(), "summarize");
/// assert_eq!(skill.properties.license.as_deref(), Some("MIT"));
/// assert_eq!(skill.properties.metadata["author"], "docs-team");
/// # Ok::<(), turul_agent_skills::BuildError>(())
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
    ///
    /// Nothing is checked until [`build`](SkillBuilder::build).
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::SkillBuilder;
    ///
    /// let skill = SkillBuilder::new("summarize", "Summarize a document.").build()?;
    /// assert_eq!(skill.body, "");
    /// # Ok::<(), turul_agent_skills::BuildError>(())
    /// ```
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

    /// Sets `license`. Every setter consumes and returns the builder; see the
    /// [type-level example](SkillBuilder).
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Sets `compatibility` (1-500 characters is a [`validate`](crate::validate) rule).
    pub fn compatibility(mut self, compatibility: impl Into<String>) -> Self {
        self.compatibility = Some(compatibility.into());
        self
    }

    /// Sets `allowed-tools`, a space-separated tool list such as `"Read Bash(jq:*)"`.
    pub fn allowed_tools(mut self, allowed_tools: impl Into<String>) -> Self {
        self.allowed_tools = Some(allowed_tools.into());
        self
    }

    /// Adds one `metadata` entry; a repeated key keeps the last value.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::SkillBuilder;
    ///
    /// let skill = SkillBuilder::new("a", "d")
    ///     .metadata("version", "1")
    ///     .metadata("version", "2")
    ///     .build()?;
    /// assert_eq!(skill.properties.metadata["version"], "2");
    /// # Ok::<(), turul_agent_skills::BuildError>(())
    /// ```
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Sets the Markdown body; it is written and reloaded byte for byte.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Builds the skill, trimming the description.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::Name`] when the name is not a legal
    /// [`SkillName`] and [`BuildError::DescriptionEmpty`] when the description
    /// is empty after trimming.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::{BuildError, SkillBuilder};
    ///
    /// let skill = SkillBuilder::new("pdf", "  Work with PDFs.  ").build()?;
    /// assert_eq!(skill.properties.description, "Work with PDFs.");
    ///
    /// assert_eq!(
    ///     SkillBuilder::new("pdf", "").build().unwrap_err(),
    ///     BuildError::DescriptionEmpty,
    /// );
    /// # Ok::<(), BuildError>(())
    /// ```
    pub fn build(self) -> Result<Skill, BuildError> {
        let name = SkillName::parse(&self.name)?;
        let description = self.description.trim();
        if description.is_empty() {
            return Err(BuildError::DescriptionEmpty);
        }
        let mut properties = SkillProperties::new(name, description.to_string());
        properties.license = self.license;
        properties.compatibility = self.compatibility;
        properties.metadata = self.metadata;
        properties.allowed_tools = self.allowed_tools;
        Ok(Skill::new(properties, self.body))
    }
}
