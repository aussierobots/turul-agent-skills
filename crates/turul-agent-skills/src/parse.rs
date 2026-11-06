use std::collections::BTreeMap;

use saphyr_parser::{Event, Parser, ScalarStyle, Span};

use crate::error::ErrorCode;
use crate::name::SkillName;

/// Deepest YAML nesting accepted, so hostile input cannot exhaust the stack.
const MAX_NESTING: usize = 64;

/// The fields the Agent Skills specification defines, as validated values.
///
/// Fields outside the specification never live here; [`load`](crate::load)
/// keeps them as extensions on the [`Skill`](crate::Skill). It is reached
/// through [`Skill::properties`](crate::Skill) and cannot be constructed
/// outside the crate.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::Skill;
///
/// let skill = Skill::builder("pdf-tools", "Work with PDFs.")
///     .license("MIT")
///     .build()?;
/// let properties = &skill.properties;
/// assert_eq!(properties.name.as_str(), "pdf-tools");
/// assert_eq!(properties.license.as_deref(), Some("MIT"));
/// assert!(properties.compatibility.is_none());
/// # Ok::<(), turul_agent_skills::BuildError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkillProperties {
    /// The skill's identity.
    pub name: SkillName,
    /// What the skill does and when to use it.
    pub description: String,
    /// The licence name or a reference to a bundled licence file.
    pub license: Option<String>,
    /// Environment requirements.
    pub compatibility: Option<String>,
    /// Free-form string-to-string metadata, iterated in key order.
    pub metadata: BTreeMap<String, String>,
    /// The space-separated pre-approved tools string.
    pub allowed_tools: Option<String>,
}

impl SkillProperties {
    /// Properties with the two required fields and every optional field absent.
    pub(crate) fn new(name: SkillName, description: impl Into<String>) -> Self {
        SkillProperties {
            name,
            description: description.into(),
            license: None,
            compatibility: None,
            metadata: BTreeMap::new(),
            allowed_tools: None,
        }
    }
}

/// A YAML value of the frontmatter subset. Every scalar is a string.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{RawValue, parse_skill_md};
///
/// let raw = parse_skill_md("---\ntags:\n  - a\n  - b\n---\n")?;
/// let RawValue::Seq(items) = &raw.entries[0].1 else { panic!("not a sequence") };
/// assert_eq!(items.len(), 2);
/// # Ok::<(), turul_agent_skills::ParseError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RawValue {
    /// A scalar. `typed` is true when it was written plain and a typed YAML
    /// parser would read it as null, a boolean or a number.
    Str {
        /// The scalar text as written, unquoted and unescaped.
        text: String,
        /// Whether the source was a plain non-string scalar.
        typed: bool,
    },
    /// A block sequence.
    Seq(Vec<RawValue>),
    /// A block mapping, in source order.
    Map(Vec<(String, RawValue)>),
}

/// A `SKILL.md` split into its top-level frontmatter entries and body.
///
/// Nothing is validated: any key may appear, in any order. Use
/// [`load`](crate::load) or [`validate`](crate::validate) for checked skills.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{RawValue, parse_skill_md};
///
/// let raw = parse_skill_md("---\nname: a\ndescription: d\n---\nBody.\n")?;
/// assert_eq!(raw.entries[0].0, "name");
/// assert_eq!(raw.body, "Body.\n");
/// assert!(raw.repaired.is_none());
///
/// // A value with an unquoted `: ` is repaired, and the original error kept.
/// let raw = parse_skill_md("---\ndescription: Use when: asked\n---\n")?;
/// assert!(raw.repaired.is_some());
/// let RawValue::Str { text, .. } = &raw.entries[0].1 else { panic!("not a scalar") };
/// assert_eq!(text, "Use when: asked");
/// # Ok::<(), turul_agent_skills::ParseError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawSkill {
    /// Every top-level key, in source order.
    pub entries: Vec<(String, RawValue)>,
    /// The text after the closing delimiter, verbatim.
    pub body: String,
    /// The original YAML error when the colon repair was needed to parse.
    pub repaired: Option<String>,
}

/// Why a document could not be parsed.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, ParseError, parse_skill_md};
///
/// let error = parse_skill_md("---\nname: a\n").unwrap_err();
/// assert_eq!(error, ParseError::FrontmatterUnclosed);
/// assert_eq!(error.code(), ErrorCode::FrontmatterUnclosed);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The first line is not `---`.
    #[error("SKILL.md must start with YAML frontmatter (---)")]
    FrontmatterMissing,
    /// No closing `---` line.
    #[error("SKILL.md frontmatter not properly closed with ---")]
    FrontmatterUnclosed,
    /// The frontmatter is not valid YAML for this format; the text says why.
    #[error("Invalid YAML in frontmatter: {0}")]
    InvalidYaml(String),
    /// The frontmatter is not a mapping.
    #[error("SKILL.md frontmatter must be a YAML mapping")]
    NotMapping,
}

impl ParseError {
    /// The stable [`ErrorCode`] for this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::{ErrorCode, ParseError};
    ///
    /// assert_eq!(ParseError::NotMapping.code(), ErrorCode::FrontmatterNotMapping);
    /// assert_eq!(ParseError::InvalidYaml("x".into()).code(), ErrorCode::YamlInvalid);
    /// ```
    pub fn code(&self) -> ErrorCode {
        match self {
            ParseError::FrontmatterMissing => ErrorCode::FrontmatterMissing,
            ParseError::FrontmatterUnclosed => ErrorCode::FrontmatterUnclosed,
            ParseError::InvalidYaml(_) => ErrorCode::YamlInvalid,
            ParseError::NotMapping => ErrorCode::FrontmatterNotMapping,
        }
    }
}

/// Splits `content` into frontmatter entries and body.
///
/// Purely syntactic: no field is required or checked here. YAML that fails to
/// parse because a plain value contains `: ` is retried with such values
/// quoted; [`RawSkill::repaired`] then holds the original error.
///
/// # Errors
///
/// - [`ParseError::FrontmatterMissing`] when the first line is not `---`.
/// - [`ParseError::FrontmatterUnclosed`] when no closing `---` line follows.
/// - [`ParseError::InvalidYaml`] when the YAML is invalid even after repair, or
///   nests deeper than the parser allows.
/// - [`ParseError::NotMapping`] when the YAML is not a mapping.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ParseError, parse_skill_md};
///
/// let raw = parse_skill_md("---\nname: a\n---\nBody.\n")?;
/// assert_eq!(raw.entries.len(), 1);
/// assert_eq!(raw.body, "Body.\n");
///
/// assert_eq!(parse_skill_md("Body only.").unwrap_err(), ParseError::FrontmatterMissing);
/// # Ok::<(), ParseError>(())
/// ```
pub fn parse_skill_md(content: &str) -> Result<RawSkill, ParseError> {
    let (yaml, body) = split_frontmatter(content)?;
    let (entries, repaired) = match parse_yaml(yaml) {
        Ok(entries) => (entries, None),
        Err(YamlFailure::NotMapping) => return Err(ParseError::NotMapping),
        Err(YamlFailure::Invalid(first)) => match parse_yaml(&quote_colon_values(yaml)) {
            Ok(entries) => (entries, Some(first)),
            Err(_) => return Err(ParseError::InvalidYaml(first)),
        },
    };
    Ok(RawSkill {
        entries,
        body: body.to_string(),
        repaired,
    })
}

fn is_delimiter(line: &str) -> bool {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line) == "---"
}

/// Returns the YAML between the delimiter lines and the text after the closing one.
fn split_frontmatter(content: &str) -> Result<(&str, &str), ParseError> {
    let mut lines = content.split_inclusive('\n');
    if !lines.next().is_some_and(is_delimiter) {
        return Err(ParseError::FrontmatterMissing);
    }
    let yaml_start = content.len() - lines.clone().map(str::len).sum::<usize>();
    let mut offset = yaml_start;
    for line in lines {
        if is_delimiter(line) {
            return Ok((
                &content[yaml_start..offset],
                &content[offset + line.len()..],
            ));
        }
        offset += line.len();
    }
    Err(ParseError::FrontmatterUnclosed)
}

enum YamlFailure {
    Invalid(String),
    NotMapping,
}

impl From<String> for YamlFailure {
    fn from(detail: String) -> Self {
        YamlFailure::Invalid(detail)
    }
}

type Events<'a> = Parser<'a, saphyr_parser::StrInput<'a>>;

fn next_event<'a>(events: &mut Events<'a>) -> Result<(Event<'a>, Span), String> {
    match events.next() {
        Some(Ok(item)) => Ok(item),
        Some(Err(error)) => Err(error.to_string()),
        None => Err("unexpected end of document".to_string()),
    }
}

/// Requires the stream to end after the root mapping's document.
fn expect_single_document(events: &mut Events<'_>) -> Result<(), String> {
    match next_event(events)?.0 {
        Event::DocumentEnd => {}
        _ => return Err("multiple documents are not allowed".to_string()),
    }
    match next_event(events)?.0 {
        Event::StreamEnd => Ok(()),
        _ => Err("multiple documents are not allowed".to_string()),
    }
}

fn parse_yaml(yaml: &str) -> Result<Vec<(String, RawValue)>, YamlFailure> {
    let source = Source::new(yaml);
    let mut events = Parser::new_from_str(yaml);
    loop {
        let (event, span) = next_event(&mut events)?;
        match event {
            Event::StreamStart | Event::DocumentStart(_) | Event::Nothing => continue,
            Event::MappingStart(anchor, tag) => {
                reject_decorated(anchor, tag.is_some())?;
                source.reject_flow(&span)?;
                let entries = build_mapping(&mut events, &source, 1)?;
                expect_single_document(&mut events)?;
                return Ok(entries);
            }
            Event::SequenceStart(..) => {
                source.reject_flow(&span)?;
                return Err(YamlFailure::NotMapping);
            }
            Event::Alias(_) => return Err(YamlFailure::Invalid("aliases are not allowed".into())),
            _ => return Err(YamlFailure::NotMapping),
        }
    }
}

fn reject_decorated(anchor: usize, tagged: bool) -> Result<(), String> {
    if anchor != 0 {
        Err("anchors are not allowed".to_string())
    } else if tagged {
        Err("tags are not allowed".to_string())
    } else {
        Ok(())
    }
}

/// The frontmatter text with the byte offset of every character, so an event's
/// character index maps to its byte in constant time.
struct Source<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> Source<'a> {
    fn new(text: &'a str) -> Self {
        Source {
            text,
            starts: text.char_indices().map(|(at, _)| at).collect(),
        }
    }

    fn reject_flow(&self, span: &Span) -> Result<(), String> {
        let at = self.starts.get(span.start.index()).copied();
        match at.and_then(|at| self.text.as_bytes().get(at)) {
            Some(b'[' | b'{') => Err("flow collections are not allowed".to_string()),
            _ => Ok(()),
        }
    }
}

fn build_value(
    events: &mut Events<'_>,
    source: &Source<'_>,
    first: (Event<'_>, Span),
    depth: usize,
) -> Result<RawValue, String> {
    if depth > MAX_NESTING {
        return Err("nesting is too deep".to_string());
    }
    let (event, span) = first;
    match event {
        Event::Scalar(text, style, anchor, tag) => {
            reject_decorated(anchor, tag.is_some())?;
            let typed =
                style == ScalarStyle::Plain && (text.is_empty() || reads_as_non_string(&text));
            Ok(RawValue::Str {
                text: text.into_owned(),
                typed,
            })
        }
        Event::Alias(_) => Err("aliases are not allowed".to_string()),
        Event::SequenceStart(anchor, tag) => {
            reject_decorated(anchor, tag.is_some())?;
            source.reject_flow(&span)?;
            let mut items = Vec::new();
            loop {
                let next = next_event(events)?;
                if matches!(next.0, Event::SequenceEnd) {
                    return Ok(RawValue::Seq(items));
                }
                items.push(build_value(events, source, next, depth + 1)?);
            }
        }
        Event::MappingStart(anchor, tag) => {
            reject_decorated(anchor, tag.is_some())?;
            source.reject_flow(&span)?;
            Ok(RawValue::Map(build_mapping(events, source, depth + 1)?))
        }
        _ => Err("unexpected YAML structure".to_string()),
    }
}

/// Reads entries up to the matching `MappingEnd`.
fn build_mapping(
    events: &mut Events<'_>,
    source: &Source<'_>,
    depth: usize,
) -> Result<Vec<(String, RawValue)>, String> {
    let mut entries: Vec<(String, RawValue)> = Vec::new();
    loop {
        let (event, _) = next_event(events)?;
        let key = match event {
            Event::MappingEnd => return Ok(entries),
            Event::Scalar(text, _, anchor, tag) => {
                reject_decorated(anchor, tag.is_some())?;
                text.into_owned()
            }
            _ => return Err("mapping keys must be scalars".to_string()),
        };
        if entries.iter().any(|(existing, _)| *existing == key) {
            return Err(format!("duplicate key '{key}'"));
        }
        let first = next_event(events)?;
        entries.push((key, build_value(events, source, first, depth)?));
    }
}

/// Whether a typed YAML parser (1.2 core or 1.1) would read the plain scalar
/// as something other than a string.
pub(crate) fn reads_as_non_string(text: &str) -> bool {
    const WORDS: &[&str] = &[
        "~", "null", "Null", "NULL", "true", "True", "TRUE", "false", "False", "FALSE", "y", "Y",
        "n", "N", "yes", "Yes", "YES", "no", "No", "NO", "on", "On", "ON", "off", "Off", "OFF",
    ];
    if WORDS.contains(&text) {
        return true;
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") || matches!(text, ".nan" | ".NaN" | ".NAN") {
        return true;
    }
    if let Some(octal) = unsigned.strip_prefix("0o") {
        return !octal.is_empty() && octal.bytes().all(|b| matches!(b, b'0'..=b'7' | b'_'));
    }
    if let Some(hex) = unsigned.strip_prefix("0x") {
        return !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit() || b == b'_');
    }
    if let Some(binary) = unsigned.strip_prefix("0b") {
        return !binary.is_empty() && binary.bytes().all(|b| matches!(b, b'0' | b'1' | b'_'));
    }
    is_decimal_number(unsigned)
        || is_underscored_number(unsigned)
        || is_sexagesimal(unsigned)
        || is_timestamp(text)
}

fn all_digits_or_underscores(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit() || b == b'_')
}

/// YAML 1.1 integers and floats with `_` separators: `1_000`, `1_0.5`.
fn is_underscored_number(s: &str) -> bool {
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(at) => (&s[..at], Some(&s[at + 1..])),
        None => (s, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    s.contains('_')
        && whole.starts_with(|c: char| c.is_ascii_digit())
        && all_digits_or_underscores(whole)
        && all_digits_or_underscores(fraction)
        && exponent.is_none_or(|e| {
            let e = e.strip_prefix(['-', '+']).unwrap_or(e);
            !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
        })
}

/// YAML 1.1 base-60 numbers: `1:30`, `190:20:30.15`.
fn is_sexagesimal(s: &str) -> bool {
    let mut parts = s.split(':');
    let first = parts.next().unwrap_or("");
    if !first.starts_with(|c: char| c.is_ascii_digit()) || !all_digits_or_underscores(first) {
        return false;
    }
    let rest: Vec<&str> = parts.collect();
    let Some((last, middle)) = rest.split_last() else {
        return false;
    };
    let (last, fraction) = last.split_once('.').unwrap_or((last, ""));
    let sexagesimal = |part: &str| {
        (1..=2).contains(&part.len())
            && part.bytes().all(|b| b.is_ascii_digit())
            && part.parse::<u8>().is_ok_and(|n| n < 60)
    };
    middle.iter().all(|part| sexagesimal(part))
        && sexagesimal(last)
        && all_digits_or_underscores(fraction)
}

/// YAML 1.1 timestamps: `2024-01-01` and `2024-1-1 10:00:00.5 +1`.
fn is_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |from: usize, min: usize, max: usize| {
        let n = b[from.min(b.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        (min..=max).contains(&n).then_some(from + n)
    };
    let at = |i: usize, c: u8| b.get(i) == Some(&c);
    let Some(i) = digits(0, 4, 4).filter(|&i| at(i, b'-')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 1, 2).filter(|&i| at(i, b'-')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 1, 2) else {
        return false;
    };
    if i == b.len() {
        return b.len() == 10;
    }
    let mut i = i;
    if matches!(b[i], b'T' | b't') {
        i += 1;
    } else {
        let spaces = b[i..]
            .iter()
            .take_while(|c| matches!(c, b' ' | b'\t'))
            .count();
        if spaces == 0 {
            return false;
        }
        i += spaces;
    }
    let Some(i) = digits(i, 1, 2).filter(|&i| at(i, b':')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 2, 2).filter(|&i| at(i, b':')) else {
        return false;
    };
    let Some(mut i) = digits(i + 1, 2, 2) else {
        return false;
    };
    if at(i, b'.') {
        i += 1 + b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
    }
    let zone = &s[i..];
    let zone = zone.trim_start_matches([' ', '\t']);
    match zone.as_bytes().first() {
        None => true,
        Some(b'Z') => zone.len() == 1,
        Some(b'-' | b'+') => {
            let offset = &zone[1..];
            let (hours, minutes) = offset.split_once(':').unwrap_or((offset, "00"));
            (1..=2).contains(&hours.len())
                && minutes.len() == 2
                && hours
                    .bytes()
                    .chain(minutes.bytes())
                    .all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// `(\.[0-9]+ | [0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`, which includes plain integers.
fn is_decimal_number(s: &str) -> bool {
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(at) => (&s[..at], Some(&s[at + 1..])),
        None => (s, None),
    };
    let mantissa_ok = match mantissa.strip_prefix('.') {
        Some(fraction) => !fraction.is_empty() && digits(fraction) == fraction.len(),
        None => {
            let whole = digits(mantissa);
            whole > 0
                && match &mantissa[whole..] {
                    "" => true,
                    rest => rest.strip_prefix('.').is_some_and(|f| digits(f) == f.len()),
                }
        }
    };
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['-', '+']).unwrap_or(e);
        !e.is_empty() && digits(e) == e.len()
    });
    mantissa_ok && exponent_ok
}

/// The colon fallback: single-quotes unindented `key: value` lines whose
/// unquoted value contains `: ` or ends with `:`.
fn quote_colon_values(yaml: &str) -> String {
    yaml.split('\n')
        .map(quote_colon_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn quote_colon_line(line: &str) -> String {
    let key_len = line
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(line.len());
    let Some(after_key) = line[key_len..].strip_prefix(':') else {
        return line.to_string();
    };
    let spaced = after_key.starts_with([' ', '\t']);
    let rest = after_key.trim_start_matches([' ', '\t']);
    // A comment starts at the first '#' after whitespace and stays outside the quotes.
    let comment_at = rest
        .match_indices('#')
        .map(|(at, _)| at)
        .find(|&at| at > 0 && rest[..at].ends_with([' ', '\t']));
    let (value, comment) = rest.split_at(comment_at.unwrap_or(rest.len()));
    let value = value.trim_end();
    let quotable = key_len > 0
        && spaced
        && !value.is_empty()
        && !value.starts_with(['"', '\'', '|', '>', '[', '{', '&', '*', '!', '#'])
        && (value.contains(": ") || value.ends_with(':'));
    if !quotable {
        return line.to_string();
    }
    let quoted = format!("{}: '{}'", &line[..key_len], value.replace('\'', "''"));
    if comment.is_empty() {
        quoted
    } else {
        format!("{quoted} {}", comment.trim_end())
    }
}
