use crate::bundle::bundled_files;
use crate::skill::Skill;

/// Most resource files [`activation`] lists before truncating.
///
/// ```
/// assert_eq!(turul_agent_skills::MAX_RESOURCE_FILES, 100);
/// ```
pub const MAX_RESOURCE_FILES: usize = 100;

/// The `<available_skills>` catalog for a model, skills in input order, or
/// `None` when there are none (an empty catalog is better omitted).
///
/// Name and description are HTML-escaped; the location is not, so it stays a
/// usable file path. A skill without a [location](Skill::location) gets no
/// `<location>` element.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{Skill, to_prompt};
///
/// let skill = Skill::builder("summarize", "Summarize <any> document.").build()?;
/// let catalog = to_prompt(&[skill]).unwrap();
/// assert!(catalog.starts_with("<available_skills>\n<skill>"));
/// assert!(catalog.contains("Summarize &lt;any&gt; document."));
///
/// assert_eq!(to_prompt(&[]), None);
/// # Ok::<(), turul_agent_skills::BuildError>(())
/// ```
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
/// skill directory and the first [`MAX_RESOURCE_FILES`] of its
/// [`bundled_files`](crate::bundled_files), with a note when the list is cut.
/// Files are listed by name and never opened. When the directory cannot be
/// read, no files are listed; call [`bundled_files`](crate::bundled_files) to
/// see the error.
///
/// A skill without a [location](Skill::location) has no directory, so only the
/// body is returned.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{Skill, activation, load};
///
/// let skill = Skill::builder("summarize", "Summarize a document.")
///     .body("Answer in three bullets.\n")
///     .build()?;
/// assert_eq!(
///     activation(&skill),
///     "<skill_content name=\"summarize\">\nAnswer in three bullets.\n</skill_content>",
/// );
///
/// // A loaded skill also lists its bundled files.
/// let tmp = tempfile::tempdir()?;
/// let file = skill.write_to(tmp.path())?;
/// let dir = file.parent().unwrap();
/// std::fs::write(dir.join("notes.txt"), "extra")?;
/// let loaded = load(dir)?.skill;
/// assert!(activation(&loaded).contains("<file>notes.txt</file>"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
        // The listing is a hint to the model; an unreadable directory lists nothing.
        let resources = bundled_files(skill).unwrap_or_default();
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
