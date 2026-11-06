use std::path::{Path, PathBuf};

use crate::builder::SkillBuilder;
use crate::parse::{RawValue, SkillProperties};

/// A loaded or constructed skill: spec fields, extension keys and the body.
///
/// Equality compares `properties`, extensions and `body`; `location` is
/// provenance, not content. Create one with [`Skill::builder`] or
/// [`load`](crate::load), turn it back into text with [`Skill::render`] and
/// write it out with [`Skill::write_to`].
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{Skill, load};
///
/// let skill = Skill::builder("summarize", "Summarize a document.")
///     .body("Answer in three bullets.\n")
///     .build()?;
///
/// let tmp = tempfile::tempdir()?;
/// let file = skill.write_to(tmp.path())?;
/// let loaded = load(file.parent().unwrap())?;
/// assert_eq!(loaded.skill, skill);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::Skill;
    ///
    /// let skill = Skill::builder("summarize", "Summarize a document.").build()?;
    /// assert_eq!(skill.properties.name.as_str(), "summarize");
    /// # Ok::<(), turul_agent_skills::BuildError>(())
    /// ```
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
    /// Only [`load`](crate::load) fills them; outside the crate they are
    /// read-only. A skill that was built has none.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::load;
    ///
    /// let tmp = tempfile::tempdir()?;
    /// let dir = tmp.path().join("a");
    /// std::fs::create_dir(&dir)?;
    /// std::fs::write(dir.join("SKILL.md"), "---\nname: a\ndescription: d\nowner: docs\n---\n")?;
    /// let skill = load(&dir)?.skill;
    /// assert_eq!(skill.extensions()[0].0, "owner");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// The list cannot be changed from outside the crate.
    ///
    /// ```compile_fail
    /// use turul_agent_skills::Skill;
    /// let mut skill = Skill::builder("a", "d").build().unwrap();
    /// skill.extensions.push(("k".to_string(), turul_agent_skills::RawValue::Seq(vec![])));
    /// ```
    pub fn extensions(&self) -> &[(String, RawValue)] {
        &self.extensions
    }

    /// The absolute, canonical path of the `SKILL.md` this skill was loaded
    /// from, or `None` for a skill that was built.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::Skill;
    ///
    /// let skill = Skill::builder("a", "d").build()?;
    /// assert!(skill.location().is_none());
    /// # Ok::<(), turul_agent_skills::BuildError>(())
    /// ```
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
