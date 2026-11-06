use std::path::{Path, PathBuf};

use crate::error::{ErrorCode, ValidationError, display_path};
use crate::skill::Skill;
use crate::validate::{check, dir_name, read_and_parse};

/// A skill that loaded, with the problems that did not stop it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// The loaded skill.
    pub skill: Skill,
    /// Problems found while loading; empty when the skill is clean.
    pub diagnostics: Vec<ValidationError>,
}

/// A skill that could not be loaded, with every error found.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{}", self.errors.first().map_or("", ValidationError::message))]
pub struct LoadSkip {
    /// Every error found. The first is the reason the skill was skipped: the
    /// errors that cause a skip come first, in `skills-ref` order (missing
    /// name, missing description, empty name, empty description, then the
    /// name identity rules), followed by the rest in report order.
    pub errors: Vec<ValidationError>,
}

/// Loads the skill in `dir` leniently: name length, directory mismatch, unknown
/// keys and bad optional fields become diagnostics; a missing or invalid name,
/// a missing description, or unparseable YAML skips the skill.
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

/// `dir/SKILL.md` if it exists, else `dir/skill.md`.
pub(crate) fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    ["SKILL.md", "skill.md"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}
