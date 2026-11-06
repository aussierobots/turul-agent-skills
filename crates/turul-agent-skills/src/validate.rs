use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::error::{ErrorCode, ValidationError, display_path};
use crate::name::{NameViolation, SkillName};
use crate::parse::{ParseError, RawSkill, RawValue, SkillProperties, parse_skill_md};

const SPEC_FIELDS: [&str; 6] = [
    "allowed-tools",
    "compatibility",
    "description",
    "license",
    "metadata",
    "name",
];
/// The message for a blank or non-string `description`.
pub(crate) const DESCRIPTION_EMPTY_MESSAGE: &str = "Field 'description' must be a non-empty string";
const MAX_NAME_CHARS: usize = 64;
const MAX_DESCRIPTION_CHARS: usize = 1024;
const MAX_COMPATIBILITY_CHARS: usize = 500;

/// Outcome of the one field check shared by `validate` and `load`.
pub(crate) struct Checked {
    /// Every field error, in report order, excluding unknown keys.
    pub errors: Vec<ValidationError>,
    /// Present when `name` and `description` are usable; optional fields that
    /// failed their check are dropped.
    pub properties: Option<SkillProperties>,
    /// Top-level keys outside the specification, in source order.
    pub extensions: Vec<(String, RawValue)>,
}

/// Checks the skill directory `dir` strictly and returns every problem found.
///
/// An empty list means the skill is valid. Path and parse problems stop the
/// check with a single error; otherwise all field errors are returned in a
/// stable order. Unlike [`load`](crate::load), nothing is tolerated: unknown
/// keys, a name that differs from the directory name, over-long fields and
/// YAML that only parses after repair are all errors.
///
/// # Errors
///
/// This function reports problems as a list of [`ValidationError`] rather than
/// a `Result`. The codes it can report are:
///
/// - [`PathNotFound`](ErrorCode::PathNotFound),
///   [`NotADirectory`](ErrorCode::NotADirectory),
///   [`SkillMdMissing`](ErrorCode::SkillMdMissing) and
///   [`ReadFailed`](ErrorCode::ReadFailed) for the path and file.
/// - [`FrontmatterMissing`](ErrorCode::FrontmatterMissing),
///   [`FrontmatterUnclosed`](ErrorCode::FrontmatterUnclosed),
///   [`YamlInvalid`](ErrorCode::YamlInvalid) and
///   [`FrontmatterNotMapping`](ErrorCode::FrontmatterNotMapping) for the
///   frontmatter.
/// - [`UnexpectedFields`](ErrorCode::UnexpectedFields) for keys outside the
///   specification.
/// - `Name*` codes ([`NameMissing`](ErrorCode::NameMissing),
///   [`NameTooLong`](ErrorCode::NameTooLong),
///   [`NameDirMismatch`](ErrorCode::NameDirMismatch) and the identity rules),
///   `Description*`, `Compatibility*`, [`LicenseNotString`](ErrorCode::LicenseNotString),
///   `Metadata*` and [`AllowedToolsNotString`](ErrorCode::AllowedToolsNotString)
///   for the fields.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, validate};
///
/// let tmp = tempfile::tempdir()?;
/// let dir = tmp.path().join("pdf-tools");
/// std::fs::create_dir(&dir)?;
/// std::fs::write(
///     dir.join("SKILL.md"),
///     "---\nname: pdf-tools\ndescription: Work with PDFs.\n---\nUse it.\n",
/// )?;
/// assert!(validate(&dir).is_empty());
///
/// // The name must match the directory.
/// std::fs::write(
///     dir.join("SKILL.md"),
///     "---\nname: other\ndescription: Work with PDFs.\n---\n",
/// )?;
/// let errors = validate(&dir);
/// assert_eq!(errors[0].code(), ErrorCode::NameDirMismatch);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn validate(dir: &Path) -> Vec<ValidationError> {
    if !dir.exists() {
        return vec![ValidationError::new(
            ErrorCode::PathNotFound,
            format!("Path does not exist: {}", display_path(dir)),
        )];
    }
    if !dir.is_dir() {
        return vec![ValidationError::new(
            ErrorCode::NotADirectory,
            format!("Not a directory: {}", display_path(dir)),
        )];
    }
    let Some(file) = find_skill_md(dir) else {
        return vec![ValidationError::new(
            ErrorCode::SkillMdMissing,
            "Missing required file: SKILL.md",
        )];
    };
    let raw = match read_and_parse(&file) {
        Ok(raw) => raw,
        Err(error) => return vec![error],
    };
    if let Some(detail) = raw.repaired {
        let error = ParseError::InvalidYaml(detail);
        return vec![ValidationError::new(error.code(), error.to_string())];
    }
    let checked = check(&raw, &dir_name(dir));
    let mut errors = Vec::new();
    if !checked.extensions.is_empty() {
        let mut keys: Vec<&str> = checked.extensions.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        errors.push(ValidationError::new(
            ErrorCode::UnexpectedFields,
            format!(
                "Unexpected fields in frontmatter: {}. Only ['allowed-tools', 'compatibility', 'description', 'license', 'metadata', 'name'] are allowed.",
                keys.join(", ")
            ),
        ));
    }
    errors.extend(checked.errors);
    errors
}

/// The final component of `dir` as given, `""` when there is none.
pub(crate) fn dir_name(dir: &Path) -> String {
    use std::path::Component;
    match dir.components().next_back() {
        Some(Component::Normal(name)) => name.to_string_lossy().into_owned(),
        Some(Component::ParentDir) => "..".to_string(),
        _ => String::new(),
    }
}

/// `dir/SKILL.md` if it exists, else `dir/skill.md`.
pub(crate) fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    ["SKILL.md", "skill.md"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

/// Reads the skill file and splits it, mapping failures to errors.
pub(crate) fn read_and_parse(file: &Path) -> Result<RawSkill, ValidationError> {
    let content = fs::read_to_string(file).map_err(|error| {
        ValidationError::new(
            ErrorCode::ReadFailed,
            format!("Failed to read {}: {error}", display_path(file)),
        )
    })?;
    parse_skill_md(&content).map_err(|error| ValidationError::new(error.code(), error.to_string()))
}

fn entry<'a>(raw: &'a RawSkill, key: &str) -> Option<&'a RawValue> {
    raw.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn text(value: &RawValue) -> Option<&str> {
    match value {
        RawValue::Str { text, .. } => Some(text),
        _ => None,
    }
}

fn error(code: ErrorCode, message: impl Into<String>) -> ValidationError {
    ValidationError::new(code, message)
}

/// The field check: reference order for `name`, `description`, `compatibility`,
/// then type checks for `license`, `metadata` and `allowed-tools`.
pub(crate) fn check(raw: &RawSkill, dir_name: &str) -> Checked {
    let mut errors = Vec::new();
    let extensions = raw
        .entries
        .iter()
        .filter(|(key, _)| !SPEC_FIELDS.contains(&key.as_str()))
        .cloned()
        .collect();

    let name = check_name(entry(raw, "name"), dir_name, &mut errors);
    let description = check_description(entry(raw, "description"), &mut errors);
    let compatibility = check_compatibility(entry(raw, "compatibility"), &mut errors);
    let license = check_string(
        entry(raw, "license"),
        "license",
        ErrorCode::LicenseNotString,
        &mut errors,
    );
    let metadata = check_metadata(entry(raw, "metadata"), &mut errors);
    let allowed_tools = check_string(
        entry(raw, "allowed-tools"),
        "allowed-tools",
        ErrorCode::AllowedToolsNotString,
        &mut errors,
    );

    let properties = name.zip(description).map(|(name, description)| {
        let mut properties = SkillProperties::new(name, description);
        properties.license = license;
        properties.compatibility = compatibility;
        properties.metadata = metadata;
        properties.allowed_tools = allowed_tools;
        properties
    });
    Checked {
        errors,
        properties,
        extensions,
    }
}

fn check_name(
    value: Option<&RawValue>,
    dir_name: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<SkillName> {
    let Some(value) = value else {
        errors.push(error(
            ErrorCode::NameMissing,
            "Missing required field in frontmatter: name",
        ));
        return None;
    };
    let Some(candidate) = text(value).filter(|t| !t.trim().is_empty()) else {
        errors.push(error(
            NameViolation::Empty.code(),
            NameViolation::Empty.message(""),
        ));
        return None;
    };
    let (normalized, violations, name) = match SkillName::parse(candidate) {
        Ok(name) => (name.as_str().to_string(), Vec::new(), Some(name)),
        Err(e) => (e.normalized, e.violations, None),
    };
    let length = normalized.chars().count();
    if length > MAX_NAME_CHARS {
        errors.push(error(
            ErrorCode::NameTooLong,
            format!("Skill name '{normalized}' exceeds {MAX_NAME_CHARS} character limit ({length} chars)"),
        ));
    }
    for violation in violations {
        errors.push(error(violation.code(), violation.message(&normalized)));
    }
    let normalized_dir: String = dir_name.nfkc().collect();
    if normalized_dir != normalized {
        errors.push(error(
            ErrorCode::NameDirMismatch,
            format!("Directory name '{dir_name}' must match skill name '{normalized}'"),
        ));
    }
    name
}

fn check_description(
    value: Option<&RawValue>,
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let Some(value) = value else {
        errors.push(error(
            ErrorCode::DescriptionMissing,
            "Missing required field in frontmatter: description",
        ));
        return None;
    };
    let Some(description) = text(value).filter(|t| !t.trim().is_empty()) else {
        errors.push(error(
            ErrorCode::DescriptionEmpty,
            DESCRIPTION_EMPTY_MESSAGE,
        ));
        return None;
    };
    let length = description.chars().count();
    if length > MAX_DESCRIPTION_CHARS {
        errors.push(error(
            ErrorCode::DescriptionTooLong,
            format!("Description exceeds {MAX_DESCRIPTION_CHARS} character limit ({length} chars)"),
        ));
    }
    Some(description.trim().to_string())
}

fn check_compatibility(
    value: Option<&RawValue>,
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let value = value?;
    let Some(compatibility) = text(value) else {
        errors.push(error(
            ErrorCode::CompatibilityNotString,
            "Field 'compatibility' must be a string",
        ));
        return None;
    };
    if compatibility.is_empty() {
        errors.push(error(
            ErrorCode::CompatibilityEmpty,
            "Field 'compatibility' must be a non-empty string",
        ));
        return None;
    }
    let length = compatibility.chars().count();
    if length > MAX_COMPATIBILITY_CHARS {
        errors.push(error(
            ErrorCode::CompatibilityTooLong,
            format!(
                "Compatibility exceeds {MAX_COMPATIBILITY_CHARS} character limit ({length} chars)"
            ),
        ));
    }
    Some(compatibility.to_string())
}

fn check_string(
    value: Option<&RawValue>,
    field: &str,
    code: ErrorCode,
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let value = value?;
    match text(value) {
        Some(text) => Some(text.to_string()),
        None => {
            errors.push(error(code, format!("Field '{field}' must be a string")));
            None
        }
    }
}

fn check_metadata(
    value: Option<&RawValue>,
    errors: &mut Vec<ValidationError>,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    match value {
        None => {}
        Some(RawValue::Str { text, typed: true }) if text.is_empty() => {}
        Some(RawValue::Map(entries)) => {
            for (key, value) in entries {
                match text(value) {
                    Some(text) => {
                        metadata.insert(key.clone(), text.to_string());
                    }
                    None => errors.push(error(
                        ErrorCode::MetadataValueNotString,
                        format!("Field 'metadata' value for key '{key}' must be a string"),
                    )),
                }
            }
        }
        Some(_) => errors.push(error(
            ErrorCode::MetadataNotMapping,
            "Field 'metadata' must be a mapping of string keys to string values",
        )),
    }
    metadata
}
