use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::parse::{RawValue, reads_as_non_string};
use crate::skill::Skill;

impl Skill {
    /// The canonical `SKILL.md` text of this skill.
    ///
    /// Spec fields come in specification order with metadata keys sorted, then
    /// extensions in insertion order, then the body byte for byte. Loading the
    /// text gives back an equal skill.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::Skill;
    ///
    /// let skill = Skill::builder("summarize", "Summarize a document.")
    ///     .body("Answer in three bullets.\n")
    ///     .build()?;
    /// assert_eq!(
    ///     skill.render(),
    ///     "---\nname: summarize\ndescription: Summarize a document.\n---\nAnswer in three bullets.\n",
    /// );
    /// # Ok::<(), turul_agent_skills::BuildError>(())
    /// ```
    pub fn render(&self) -> String {
        let p = &self.properties;
        let mut out = String::from("---\n");
        write_text(&mut out, 0, "name", p.name.as_str());
        write_text(&mut out, 0, "description", &p.description);
        for (key, value) in [("license", &p.license), ("compatibility", &p.compatibility)] {
            if let Some(value) = value {
                write_text(&mut out, 0, key, value);
            }
        }
        if !p.metadata.is_empty() {
            out.push_str("metadata:\n");
            for (key, value) in &p.metadata {
                write_text(&mut out, 2, key, value);
            }
        }
        if let Some(tools) = &p.allowed_tools {
            write_text(&mut out, 0, "allowed-tools", tools);
        }
        for (key, value) in &self.extensions {
            write_entry(&mut out, 0, key, value);
        }
        out.push_str("---\n");
        out.push_str(&self.body);
        out
    }

    /// Writes [`render`](Skill::render) to `<parent>/<name>/SKILL.md`, creating
    /// directories, and returns the file path. Nothing else is written.
    ///
    /// # Errors
    ///
    /// Returns the [`io::Error`] from creating the directory or writing the
    /// file, for example when `parent` is not writable or a file is in the way.
    ///
    /// # Examples
    ///
    /// ```
    /// use turul_agent_skills::Skill;
    ///
    /// let tmp = tempfile::tempdir()?;
    /// let skill = Skill::builder("summarize", "Summarize a document.").build()?;
    /// let file = skill.write_to(tmp.path())?;
    /// assert!(file.ends_with("summarize/SKILL.md"));
    ///
    /// // A regular file where the directory should go is an I/O error.
    /// let blocked = tempfile::NamedTempFile::new()?;
    /// assert!(skill.write_to(blocked.path()).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_to(&self, parent: &Path) -> io::Result<PathBuf> {
        let dir = parent.join(self.properties.name.as_str());
        fs::create_dir_all(&dir)?;
        let file = dir.join("SKILL.md");
        fs::write(&file, self.render())?;
        Ok(file)
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

/// The longest key YAML allows in the `key:` form, in characters.
const MAX_SIMPLE_KEY: usize = 1024;

/// `key:`, or `? key` and `:` on two lines when the key is too long for the
/// `key:` form. The value follows.
fn push_key(out: &mut String, indent: usize, key: &str) {
    push_indent(out, indent);
    if key.chars().count() > MAX_SIMPLE_KEY {
        out.push_str("? ");
        out.push_str(&quote(key));
        out.push('\n');
        push_indent(out, indent);
    } else {
        out.push_str(&quote(key));
    }
    out.push(':');
}

/// `key: <quoted text>`, a string by every YAML parser.
fn write_text(out: &mut String, indent: usize, key: &str, text: &str) {
    push_key(out, indent, key);
    out.push(' ');
    out.push_str(&quote(text));
    out.push('\n');
}

/// The scalar as written: `typed` scalars verbatim, the rest quoted when needed.
fn scalar(text: &str, typed: bool) -> String {
    if typed { text.to_string() } else { quote(text) }
}

fn write_entry(out: &mut String, indent: usize, key: &str, value: &RawValue) {
    push_key(out, indent, key);
    match value {
        RawValue::Str { text, typed } => {
            let scalar = scalar(text, *typed);
            if !scalar.is_empty() {
                out.push(' ');
                out.push_str(&scalar);
            }
            out.push('\n');
        }
        RawValue::Seq(items) => {
            out.push('\n');
            for item in items {
                write_item(out, indent + 2, item);
            }
        }
        RawValue::Map(entries) => {
            out.push('\n');
            for (key, value) in entries {
                write_entry(out, indent + 2, key, value);
            }
        }
    }
}

fn write_item(out: &mut String, indent: usize, item: &RawValue) {
    push_indent(out, indent);
    out.push('-');
    match item {
        RawValue::Str { text, typed } => {
            let scalar = scalar(text, *typed);
            if !scalar.is_empty() {
                out.push(' ');
                out.push_str(&scalar);
            }
            out.push('\n');
        }
        RawValue::Seq(items) => {
            out.push('\n');
            for item in items {
                write_item(out, indent + 2, item);
            }
        }
        RawValue::Map(entries) => {
            out.push('\n');
            for (key, value) in entries {
                write_entry(out, indent + 2, key, value);
            }
        }
    }
}

fn needs_double_quotes(c: char) -> bool {
    c.is_control() || matches!(c, '\u{feff}' | '\u{2028}' | '\u{2029}')
}

/// Plain when that reads back as the same string everywhere, else single-quoted,
/// or double-quoted when the text holds control characters.
fn quote(text: &str) -> String {
    if text.chars().any(needs_double_quotes) {
        return double_quoted(text);
    }
    let first = text.chars().next();
    let single = text.is_empty()
        || text.starts_with(char::is_whitespace)
        || text.ends_with(char::is_whitespace)
        || first.is_some_and(|c| "-?:,[]{}#&*!|>'\"%@`".contains(c))
        || text.starts_with("...")
        || text.contains(": ")
        || text.contains(" #")
        || text.ends_with(':')
        || reads_as_non_string(text);
    if single {
        format!("'{}'", text.replace('\'', "''"))
    } else {
        text.to_string()
    }
}

fn double_quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if needs_double_quotes(c) && u32::from(c) <= 0xff => {
                out.push_str(&format!("\\x{:02x}", u32::from(c)));
            }
            c if needs_double_quotes(c) => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
