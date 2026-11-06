use std::fs;
use std::path::Path;

use crate::discover::MAX_SCAN_DEPTH;
use crate::skill::Skill;

/// Most resource files `activation` lists before truncating.
pub const MAX_RESOURCE_FILES: usize = 100;

/// The `<available_skills>` catalog for a model, skills in input order, or
/// `None` when there are none (an empty catalog is better omitted).
///
/// Name and description are HTML-escaped; the location is not, so it stays a
/// usable file path. A skill without a location gets no `<location>` element.
pub fn to_prompt(skills: &[Skill]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }
    let mut lines: Vec<String> = vec!["<available_skills>".to_string()];
    for skill in skills {
        lines.push("<skill>".to_string());
        lines.push("<name>".to_string());
        lines.push(escape_html(skill.properties.name.as_str()));
        lines.push("</name>".to_string());
        lines.push("<description>".to_string());
        lines.push(escape_html(&skill.properties.description));
        lines.push("</description>".to_string());
        if let Some(location) = skill.location() {
            lines.push("<location>".to_string());
            lines.push(location.to_string_lossy().into_owned());
            lines.push("</location>".to_string());
        }
        lines.push("</skill>".to_string());
    }
    lines.push("</available_skills>".to_string());
    Some(lines.join("\n"))
}

/// The text a client returns when a skill is activated: the trimmed body, the
/// skill directory and a capped list of bundled files. Files are listed by
/// name and never opened.
pub fn activation(skill: &Skill) -> String {
    let mut out = format!(
        "<skill_content name=\"{}\">\n{}\n",
        escape_html(skill.properties.name.as_str()),
        skill.body.trim()
    );
    if let Some(location) = skill.location() {
        let dir = location.parent().unwrap_or(location);
        out.push_str(&format!(
            "\nSkill directory: {}\nRelative paths in this skill are relative to the skill directory.\n",
            dir.to_string_lossy()
        ));
        let resources = list_resources(dir, location);
        if !resources.is_empty() {
            out.push_str("\n<skill_resources>\n");
            for file in resources.iter().take(MAX_RESOURCE_FILES) {
                out.push_str(&format!("  <file>{file}</file>\n"));
            }
            if resources.len() > MAX_RESOURCE_FILES {
                out.push_str(&format!(
                    "  (listing truncated at {MAX_RESOURCE_FILES} files)\n"
                ));
            }
            out.push_str("</skill_resources>\n");
        }
    }
    out.push_str("</skill_content>");
    out
}

/// Python `html.escape(s, quote=True)`.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Files under `dir` except `skill_file`, as `/`-joined relative paths in byte order.
/// Symbolic links count as files and are not followed.
fn list_resources(dir: &Path, skill_file: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect(dir, "", 0, skill_file, &mut files);
    files.sort_unstable();
    files
}

fn collect(dir: &Path, prefix: &str, depth: usize, skill_file: &Path, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            let skipped = name == ".git" || name == "node_modules";
            if !skipped && depth < MAX_SCAN_DEPTH {
                collect(
                    &entry.path(),
                    &format!("{relative}/"),
                    depth + 1,
                    skill_file,
                    files,
                );
            }
        } else if (kind.is_file() || kind.is_symlink()) && entry.path() != skill_file {
            files.push(relative);
        }
    }
}
