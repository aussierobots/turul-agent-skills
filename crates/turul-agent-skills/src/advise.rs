use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::discover::SKIPPED_DIRS;
use crate::error::{ErrorCode, ValidationError, display_path};
use crate::prompt::MAX_RESOURCE_FILES;
use crate::skill::Skill;

/// Lines in `SKILL.md` above which [`advise`] reports
/// [`ErrorCode::SkillMdTooLong`].
///
/// ```
/// assert_eq!(turul_agent_skills::MAX_RECOMMENDED_LINES, 500);
/// ```
pub const MAX_RECOMMENDED_LINES: usize = 500;

/// Reports the two measurable authoring recommendations of the specification's
/// progressive-disclosure guidance. Advisories are not validation: they never
/// make [`validate`](crate::validate) or [`load`](crate::load) fail, and judging
/// scope, gotchas or description quality is left to review.
///
/// - [`ErrorCode::SkillMdTooLong`]: the rendered `SKILL.md` ([`Skill::render`])
///   has more than [`MAX_RECOMMENDED_LINES`] lines.
/// - [`ErrorCode::NestedReference`]: a file the body references by a relative
///   Markdown link or backticked path, inside the skill directory, itself
///   references another file in the skill directory.
///
/// References in fenced code blocks, URLs, anchors, absolute paths, paths with
/// `..` and paths that resolve outside the skill directory (by symbolic link
/// too) are ignored. Only `.md`, `.markdown` and `.txt` files are read for
/// second-level references, at most [`MAX_RESOURCE_FILES`] of them. A skill
/// without a [location](Skill::location) gets only the line check. The result
/// is in a fixed order: the line check, then chains by first-level file in body
/// order and target in file order.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{ErrorCode, Skill, advise};
///
/// let tidy = Skill::builder("tidy", "A short skill.").body("Do it.\n").build()?;
/// assert!(advise(&tidy).is_empty());
///
/// let long = Skill::builder("long", "A long skill.").body("line\n".repeat(600)).build()?;
/// let advisories = advise(&long);
/// assert_eq!(advisories[0].code(), ErrorCode::SkillMdTooLong);
/// assert!(advisories[0].message().contains("500"));
/// # Ok::<(), turul_agent_skills::BuildError>(())
/// ```
pub fn advise(skill: &Skill) -> Vec<ValidationError> {
    let mut advisories = Vec::new();
    let lines = skill.render().lines().count();
    if lines > MAX_RECOMMENDED_LINES {
        advisories.push(ValidationError::new(
            ErrorCode::SkillMdTooLong,
            format!(
                "SKILL.md has {lines} lines; the specification recommends at most {MAX_RECOMMENDED_LINES}"
            ),
        ));
    }
    if let Some(location) = skill.location() {
        nested_references(skill, location, &mut advisories);
    }
    advisories
}

fn nested_references(skill: &Skill, location: &Path, advisories: &mut Vec<ValidationError>) {
    let Some(root) = location.parent().and_then(|dir| fs::canonicalize(dir).ok()) else {
        return;
    };
    let skill_file = fs::canonicalize(location).unwrap_or_else(|_| location.to_path_buf());
    let mut read = 0;
    for first in resolve(&skill.body, &[&root], &root, &skill_file) {
        if !is_text(&first) {
            continue;
        }
        if read >= MAX_RESOURCE_FILES {
            break;
        }
        read += 1;
        let Ok(text) = fs::read_to_string(&first) else {
            continue;
        };
        let base = first.parent().unwrap_or(&root);
        for second in resolve(&text, &[base, &root], &root, &skill_file) {
            if second != first {
                advisories.push(ValidationError::new(
                    ErrorCode::NestedReference,
                    format!(
                        "{} references {}; keep file references one level deep from SKILL.md",
                        relative(&first, &root),
                        relative(&second, &root)
                    ),
                ));
            }
        }
    }
}

fn relative(path: &Path, root: &Path) -> String {
    display_path(path.strip_prefix(root).unwrap_or(path))
}

fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["md", "markdown", "txt"].contains(&e.to_ascii_lowercase().as_str()))
}

/// The distinct existing files inside `root` that `text` references, in
/// order of appearance. Each target is tried against `bases` in turn.
fn resolve(text: &str, bases: &[&Path], root: &Path, skill_file: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for target in targets(text) {
        for base in bases {
            let Ok(path) = fs::canonicalize(base.join(&target)) else {
                continue;
            };
            let Ok(inside) = path.strip_prefix(root) else {
                continue;
            };
            let skipped = inside
                .components()
                .any(|c| SKIPPED_DIRS.iter().any(|d| c.as_os_str() == *d));
            if path.is_file() && path != skill_file && !skipped {
                if !found.contains(&path) {
                    found.push(path);
                }
                break;
            }
        }
    }
    found
}

/// Relative path candidates from Markdown links and single-backtick spans,
/// outside fenced code blocks.
fn targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        for part in line.split("](").skip(1) {
            let inner = part.split(')').next().unwrap_or("");
            out.push(inner.trim_start_matches('<').to_string());
        }
        for (i, span) in line.split('`').enumerate() {
            if i % 2 == 1 {
                out.push(span.to_string());
            }
        }
    }
    out.into_iter().filter_map(|t| clean(&t)).collect()
}

/// The path part of a candidate (a link title or trailing prose is dropped), or
/// `None` when it is not a relative in-skill path.
fn clean(candidate: &str) -> Option<String> {
    let first = candidate.split_whitespace().next()?;
    let path = first.split(['#', '?']).next()?.trim_end_matches('>');
    let p = Path::new(path);
    let escapes = p.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    (!path.is_empty() && !path.contains(':') && !escapes).then(|| path.to_string())
}
