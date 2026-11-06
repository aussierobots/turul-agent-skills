use std::path::Path;

use crate::error::{ErrorCode, ValidationError, display_path};
use crate::skill::Skill;
use crate::validate::{check, dir_name, find_skill_md, read_and_parse};

/// A skill that loaded, with the problems that did not stop it.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, load};
///
/// let tmp = tempfile::tempdir()?;
/// let dir = tmp.path().join("pdf-tools");
/// std::fs::create_dir(&dir)?;
/// std::fs::write(
///     dir.join("SKILL.md"),
///     "---\nname: pdf-tools\ndescription: Work with PDFs.\nowner: docs\n---\nUse it.\n",
/// )?;
///
/// let loaded = load(&dir)?;
/// assert_eq!(loaded.skill.properties.name.as_str(), "pdf-tools");
/// assert_eq!(loaded.diagnostics[0].code(), ErrorCode::UnknownField);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Loaded {
    /// The loaded skill.
    pub skill: Skill,
    /// Problems found while loading; empty when the skill is clean.
    pub diagnostics: Vec<ValidationError>,
}

/// A skill that could not be loaded, with every error found.
///
/// Its [`Display`](std::fmt::Display) text is the message of the first error.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, load};
///
/// let tmp = tempfile::tempdir()?;
/// let skip = load(tmp.path()).unwrap_err();
/// assert_eq!(skip.errors[0].code(), ErrorCode::SkillMdMissing);
/// assert_eq!(skip.to_string(), skip.errors[0].message());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{}", self.errors.first().map_or("", ValidationError::message))]
#[non_exhaustive]
pub struct LoadSkip {
    /// Every error found. The first is the reason the skill was skipped: the
    /// errors that cause a skip come first, in `skills-ref` order (missing
    /// name, missing description, empty name, empty description, then the
    /// name identity rules), followed by the rest in report order.
    pub errors: Vec<ValidationError>,
}

/// Loads the skill in `dir` leniently, keeping what is usable.
///
/// Name length, a name that differs from the directory, unknown keys, bad
/// optional fields and repaired YAML become [`Loaded::diagnostics`]. A missing
/// or invalid name, a missing description, or unparseable YAML skips the skill.
/// Use [`validate`](crate::validate) instead to enforce every rule strictly.
///
/// The returned [`Skill::location`] is the canonical path of the `SKILL.md`
/// file. `dir` may hold `SKILL.md` or `skill.md`; the upper-case name wins.
///
/// # Errors
///
/// Returns a [`LoadSkip`] whose first error is the reason, with one of these
/// [`ErrorCode`] values:
///
/// - [`SkillMdMissing`](ErrorCode::SkillMdMissing) when `dir` holds no skill
///   file or does not exist.
/// - [`ReadFailed`](ErrorCode::ReadFailed) when the file is not readable UTF-8.
/// - [`FrontmatterMissing`](ErrorCode::FrontmatterMissing),
///   [`FrontmatterUnclosed`](ErrorCode::FrontmatterUnclosed),
///   [`YamlInvalid`](ErrorCode::YamlInvalid) or
///   [`FrontmatterNotMapping`](ErrorCode::FrontmatterNotMapping) when the
///   frontmatter cannot be parsed.
/// - [`NameMissing`](ErrorCode::NameMissing),
///   [`NameEmpty`](ErrorCode::NameEmpty) or a name identity code such as
///   [`NameNotLowercase`](ErrorCode::NameNotLowercase), and
///   [`DescriptionMissing`](ErrorCode::DescriptionMissing) or
///   [`DescriptionEmpty`](ErrorCode::DescriptionEmpty), when the required
///   fields are unusable.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, load};
///
/// let tmp = tempfile::tempdir()?;
/// let dir = tmp.path().join("pdf-tools");
/// std::fs::create_dir(&dir)?;
/// std::fs::write(
///     dir.join("SKILL.md"),
///     "---\nname: pdf-tools\ndescription: Work with PDFs.\n---\nUse it.\n",
/// )?;
///
/// let loaded = load(&dir)?;
/// assert!(loaded.diagnostics.is_empty());
/// assert_eq!(loaded.skill.body, "Use it.\n");
///
/// // A skill with no description is skipped.
/// std::fs::write(dir.join("SKILL.md"), "---\nname: pdf-tools\n---\n")?;
/// let skip = load(&dir).unwrap_err();
/// assert_eq!(skip.errors[0].code(), ErrorCode::DescriptionMissing);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load(dir: &Path) -> Result<Loaded, LoadSkip> {
    let skip = |error: ValidationError| LoadSkip {
        errors: vec![error],
    };
    let Some(file) = find_skill_md(dir) else {
        return Err(skip(ValidationError::new(
            ErrorCode::SkillMdMissing,
            format!("SKILL.md not found in {}", display_path(dir)),
        )));
    };
    let raw = read_and_parse(&file).map_err(skip)?;

    let checked = check(&raw, &dir_name(dir));
    let mut diagnostics = Vec::new();
    if let Some(detail) = &raw.repaired {
        diagnostics.push(ValidationError::new(
            ErrorCode::YamlRepaired,
            format!("Frontmatter YAML was repaired by quoting values that contain ': ' ({detail})"),
        ));
    }
    for (key, _) in &checked.extensions {
        diagnostics.push(ValidationError::new(
            ErrorCode::UnknownField,
            format!("Unknown frontmatter field '{key}' kept as an extension"),
        ));
    }
    diagnostics.extend(checked.errors.iter().cloned());

    let Some(properties) = checked.properties else {
        let mut errors = checked.errors;
        errors.sort_by_key(|e| skip_rank(e.code()));
        return Err(LoadSkip { errors });
    };
    let location = dir
        .canonicalize()
        .ok()
        .and_then(|canonical| file.file_name().map(|name| canonical.join(name)));
    let mut skill = Skill::new(properties, raw.body);
    skill.extensions = checked.extensions;
    skill.location = location;
    Ok(Loaded { skill, diagnostics })
}

/// Position of an error among the reasons a skill is skipped; the rest sort last.
fn skip_rank(code: ErrorCode) -> u8 {
    match code {
        ErrorCode::NameMissing => 0,
        ErrorCode::DescriptionMissing => 1,
        ErrorCode::NameEmpty => 2,
        ErrorCode::DescriptionEmpty => 3,
        ErrorCode::NameNotLowercase
        | ErrorCode::NameHyphenEdge
        | ErrorCode::NameConsecutiveHyphens
        | ErrorCode::NameInvalidChars => 4,
        _ => 5,
    }
}
