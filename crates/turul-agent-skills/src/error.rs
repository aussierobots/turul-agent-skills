use std::path::{Component, Path};

macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => $code:literal,)+) => {
        /// A stable, machine-readable problem kind.
        ///
        /// Every [`ValidationError`] carries one. The kebab-case text from
        /// [`as_str`](ErrorCode::as_str) is the form to log, match on in
        /// scripts, or compare in tests. The enum is `#[non_exhaustive]`, so a
        /// `match` needs a wildcard arm.
        ///
        /// # Examples
        ///
        /// ```
        /// use turul_agent_skills::ErrorCode;
        ///
        /// assert_eq!(ErrorCode::NameMissing.as_str(), "name-missing");
        /// assert_eq!(ErrorCode::NameMissing.to_string(), "name-missing");
        ///
        /// let advice = match ErrorCode::NameTooLong {
        ///     ErrorCode::NameTooLong => "shorten the name",
        ///     _ => "see the message",
        /// };
        /// assert_eq!(advice, "shorten the name");
        /// ```
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum ErrorCode {
            $($(#[$doc])* $variant,)+
        }

        impl std::fmt::Display for ErrorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ErrorCode {
            /// The stable kebab-case code, such as `"name-missing"`.
            ///
            /// # Examples
            ///
            /// ```
            /// use turul_agent_skills::ErrorCode;
            ///
            /// assert_eq!(ErrorCode::SkillMdMissing.as_str(), "skill-md-missing");
            /// ```
            pub fn as_str(self) -> &'static str {
                match self {
                    $(ErrorCode::$variant => $code,)+
                }
            }
        }
    };
}

error_codes! {
    /// The path does not exist.
    PathNotFound => "path-not-found",
    /// The path is not a directory.
    NotADirectory => "not-a-directory",
    /// The directory holds neither `SKILL.md` nor `skill.md`.
    SkillMdMissing => "skill-md-missing",
    /// The skill file could not be read as UTF-8 text.
    ReadFailed => "read-failed",
    /// The file does not start with a `---` line.
    FrontmatterMissing => "frontmatter-missing",
    /// No closing `---` line follows the opening one.
    FrontmatterUnclosed => "frontmatter-unclosed",
    /// The frontmatter is not valid YAML for this format.
    YamlInvalid => "yaml-invalid",
    /// The frontmatter is valid YAML but not a mapping.
    FrontmatterNotMapping => "frontmatter-not-mapping",
    /// Keys outside the specification are present.
    UnexpectedFields => "unexpected-fields",
    /// `name` is absent.
    NameMissing => "name-missing",
    /// `name` is empty or not a string.
    NameEmpty => "name-empty",
    /// `name` is longer than 64 characters.
    NameTooLong => "name-too-long",
    /// `name` is not lowercase.
    NameNotLowercase => "name-not-lowercase",
    /// `name` starts or ends with a hyphen.
    NameHyphenEdge => "name-hyphen-edge",
    /// `name` contains two hyphens in a row.
    NameConsecutiveHyphens => "name-consecutive-hyphens",
    /// `name` contains a character that is not a letter, digit or hyphen.
    NameInvalidChars => "name-invalid-chars",
    /// `name` differs from the directory name.
    NameDirMismatch => "name-dir-mismatch",
    /// `description` is absent.
    DescriptionMissing => "description-missing",
    /// `description` is empty or not a string.
    DescriptionEmpty => "description-empty",
    /// `description` is longer than 1024 characters.
    DescriptionTooLong => "description-too-long",
    /// `compatibility` is not a string.
    CompatibilityNotString => "compatibility-not-string",
    /// `compatibility` is empty.
    CompatibilityEmpty => "compatibility-empty",
    /// `compatibility` is longer than 500 characters.
    CompatibilityTooLong => "compatibility-too-long",
    /// `license` is not a string.
    LicenseNotString => "license-not-string",
    /// `metadata` is not a mapping.
    MetadataNotMapping => "metadata-not-mapping",
    /// A `metadata` value is not a string.
    MetadataValueNotString => "metadata-value-not-string",
    /// `allowed-tools` is not a string.
    AllowedToolsNotString => "allowed-tools-not-string",
    /// The YAML needed the colon repair to parse.
    YamlRepaired => "yaml-repaired",
    /// A key outside the specification was kept as an extension.
    UnknownField => "unknown-field",
    /// A skill lost a name collision to a higher-ranked one.
    SkillShadowed => "skill-shadowed",
    /// A scope hit the directory cap and was cut short.
    ScanLimitReached => "scan-limit-reached",
    /// A symbolic link resolves outside the scope root and was not followed.
    SymlinkEscape => "symlink-escape",
    /// A directory could not be read.
    ScanIo => "scan-io",
}

/// One problem found in a skill, with a stable [`ErrorCode`] and a readable message.
///
/// Its [`Display`](std::fmt::Display) text is the message. Values come from
/// [`validate`](crate::validate), [`load`](crate::load) and
/// [`discover`](crate::discover); they cannot be constructed outside the crate.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, validate};
///
/// let tmp = tempfile::tempdir()?;
/// let errors = validate(tmp.path());
/// assert_eq!(errors[0].code(), ErrorCode::SkillMdMissing);
/// assert_eq!(errors[0].message(), "Missing required file: SKILL.md");
/// assert_eq!(errors[0].to_string(), errors[0].message());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ValidationError {
    code: ErrorCode,
    message: String,
}

impl ValidationError {
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ValidationError {
            code,
            message: message.into(),
        }
    }

    /// The stable problem kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::{ErrorCode, validate};
    ///
    /// let errors = validate(std::path::Path::new("no/such/dir"));
    /// assert_eq!(errors[0].code(), ErrorCode::PathNotFound);
    /// ```
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// The human-readable text.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::validate;
    ///
    /// let errors = validate(std::path::Path::new("no/such/dir"));
    /// assert_eq!(errors[0].message(), "Path does not exist: no/such/dir");
    /// ```
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Formats a path the way messages and the command line print it: `/`-joined,
/// `.` components dropped, and an empty path as `.`.
///
/// This is the form used inside [`ValidationError`] messages, so tests and
/// tools can reproduce them.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use turul_agent_skills::display_path;
///
/// assert_eq!(display_path(Path::new("./skills/./pdf")), "skills/pdf");
/// assert_eq!(display_path(Path::new("")), ".");
/// ```
pub fn display_path(path: &Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        let part = match component {
            Component::CurDir => continue,
            Component::RootDir => "",
            other => &*other.as_os_str().to_string_lossy(),
        };
        if !out.is_empty() && !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(part);
        if matches!(component, Component::RootDir) {
            out.push('/');
        }
    }
    if out.is_empty() { ".".to_string() } else { out }
}
