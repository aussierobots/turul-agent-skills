//! Command line for Agent Skills (`SKILL.md`), compatible with `skills-ref`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use turul_agent_skills::{
    ErrorCode, Skill, SkillProperties, ValidationError, display_path, load, to_prompt, validate,
};

#[derive(Parser)]
#[command(name = "turul-skills", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a skill directory
    Validate {
        /// Skill directory or its SKILL.md
        #[arg(value_name = "SKILL_PATH")]
        path: PathBuf,
    },
    /// Print a skill's properties as JSON
    ReadProperties {
        /// Skill directory or its SKILL.md
        #[arg(value_name = "SKILL_PATH")]
        path: PathBuf,
    },
    /// Print the <available_skills> catalog for skill directories
    ToPrompt {
        /// Skill directories or their SKILL.md files
        #[arg(value_name = "SKILL_PATHS", required = true)]
        paths: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (name, arg, paths) = match &cli.command {
        Command::Validate { path } => ("validate", "SKILL_PATH", vec![path]),
        Command::ReadProperties { path } => ("read-properties", "SKILL_PATH", vec![path]),
        Command::ToPrompt { paths } => ("to-prompt", "SKILL_PATHS", paths.iter().collect()),
    };
    for path in paths {
        if !path.exists() {
            eprintln!(
                "Usage: turul-skills {name} [OPTIONS] {arg}\nTry 'turul-skills {name} --help' for help.\n\nError: Invalid value for '{arg}': Path '{}' does not exist.",
                path.display()
            );
            return ExitCode::from(2);
        }
    }
    match cli.command {
        Command::Validate { path } => {
            let dir = skill_dir(&path);
            let errors = validate(&dir);
            let shown = display_path(&dir);
            if errors.is_empty() {
                println!("Valid skill: {shown}");
                return ExitCode::SUCCESS;
            }
            eprintln!("Validation failed for {shown}:");
            for error in errors {
                eprintln!("  - {}", error.message());
            }
            ExitCode::FAILURE
        }
        Command::ReadProperties { path } => match load_skill(&skill_dir(&path)) {
            Ok(skill) => {
                println!("{}", properties_json(&skill.properties));
                ExitCode::SUCCESS
            }
            Err(error) => fail(&error),
        },
        Command::ToPrompt { paths } => {
            let mut skills = Vec::new();
            for path in &paths {
                let dir = skill_dir(path);
                let dir = dir.canonicalize().unwrap_or(dir);
                match load_skill(&dir) {
                    Ok(skill) => skills.push(skill),
                    Err(error) => return fail(&error),
                }
            }
            println!("{}", to_prompt(&skills).unwrap_or_default());
            ExitCode::SUCCESS
        }
    }
}

/// A `skill.md` file (any case) stands for its directory.
fn skill_dir(path: &Path) -> PathBuf {
    let is_skill_md = path.is_file()
        && path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_lowercase() == "skill.md");
    if !is_skill_md {
        return path.to_path_buf();
    }
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Loads like `skills-ref`: a document whose YAML only parses after the
/// library's colon repair is the strict YAML error, not a skill.
fn load_skill(dir: &Path) -> Result<Skill, Option<ValidationError>> {
    let loaded = load(dir).map_err(|skip| skip.errors.into_iter().next())?;
    if loaded
        .diagnostics
        .iter()
        .any(|d| d.code() == ErrorCode::YamlRepaired)
    {
        return Err(validate(dir).into_iter().next());
    }
    Ok(loaded.skill)
}

fn fail(error: &Option<ValidationError>) -> ExitCode {
    if let Some(error) = error {
        eprintln!("Error: {}", error.message());
    }
    ExitCode::FAILURE
}

/// Python `json.dumps(properties, indent=2)` with `ensure_ascii`.
fn properties_json(p: &SkillProperties) -> String {
    let mut fields = vec![
        ("name", json_string(p.name.as_str())),
        ("description", json_string(&p.description)),
    ];
    for (key, value) in [
        ("license", &p.license),
        ("compatibility", &p.compatibility),
        ("allowed-tools", &p.allowed_tools),
    ] {
        if let Some(value) = value {
            fields.push((key, json_string(value)));
        }
    }
    let mut lines: Vec<String> = fields
        .into_iter()
        .map(|(key, value)| format!("  {}: {value}", json_string(key)))
        .collect();
    if !p.metadata.is_empty() {
        let entries: Vec<String> = p
            .metadata
            .iter()
            .map(|(k, v)| format!("    {}: {}", json_string(k), json_string(v)))
            .collect();
        lines.push(format!("  \"metadata\": {{\n{}\n  }}", entries.join(",\n")));
    }
    format!("{{\n{}\n}}", lines.join(",\n"))
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e && (c as u32) != 0x7f => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
