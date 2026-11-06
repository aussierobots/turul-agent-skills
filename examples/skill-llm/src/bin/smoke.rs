//! Exercises every public capability of `turul-agent-skills` with one assertion
//! per step: names, parsing, validation, loading, building, writing, the catalog
//! prompt, activation, discovery and paths, then three steps that put a model in
//! the loop. Prints `ok <step>` per step and exits 1 on the first failure.
//!
//! Offline stub by default; `RUN_OLLAMA_SMOKE=1` (or `OLLAMA_BASE_URL`, with
//! `OLLAMA_MODEL`) uses a live Ollama model. Model steps assert structure, never wording.
//!
//! `cargo run -p skill-llm --bin smoke`

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use skill_llm::{Stub, client};
use turul_agent_skills::prelude::*;
use turul_agent_skills::{
    ErrorCode, NameViolation, ParseError, ValidationError, display_path, parse_skill_md,
};
use turul_llm_core::{CompletionRequest, ExecutionHints, LlmClient};

type Step = Result<(), Box<dyn Error>>;

macro_rules! ensure {
    ($cond:expr) => {
        if !$cond {
            return Err(format!("assertion failed: {}", stringify!($cond)).into());
        }
    };
    ($cond:expr, $($detail:tt)+) => {
        if !$cond {
            return Err(format!(
                "assertion failed: {}: {}",
                stringify!($cond),
                format!($($detail)+)
            )
            .into());
        }
    };
}

fn report(name: &str, result: Step) {
    match result {
        Ok(()) => println!("ok {name}"),
        Err(error) => {
            eprintln!("FAIL {name}: {error}");
            std::process::exit(1);
        }
    }
}

fn skills_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/skills"))
}

fn reference_root() -> PathBuf {
    skills_root().join("reference")
}

/// Writes `<root>/<dir>/SKILL.md` and returns the directory.
fn write_raw(root: &Path, dir: &str, content: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = root.join(dir);
    fs::create_dir_all(&path)?;
    fs::write(path.join("SKILL.md"), content)?;
    Ok(path)
}

fn codes<'a>(errors: impl IntoIterator<Item = &'a ValidationError>) -> Vec<ErrorCode> {
    errors.into_iter().map(|e| e.code()).collect()
}

fn name_parse_trims() -> Step {
    let name = SkillName::parse(" greet-user ")?;
    ensure!(name.as_str() == "greet-user", "{name}");
    Ok(())
}

fn name_parse_nfkc() -> Step {
    let name = SkillName::parse("ｇreet")?;
    ensure!(name.as_str() == "greet", "{name}");
    Ok(())
}

fn name_error_lists_violations() -> Step {
    let Err(error) = SkillName::parse("-My--Skill_") else {
        return Err("an invalid name was accepted".into());
    };
    ensure!(
        error.violations
            == [
                NameViolation::NotLowercase,
                NameViolation::HyphenEdge,
                NameViolation::ConsecutiveHyphens,
                NameViolation::InvalidChars,
            ],
        "{:?}",
        error.violations
    );
    ensure!(!error.to_string().is_empty());
    Ok(())
}

fn parse_skill_md_splits_and_rejects() -> Step {
    let raw = parse_skill_md("---\nname: a\ndescription: d\n---\nBody\n")?;
    let keys: Vec<&str> = raw.entries.iter().map(|(k, _)| k.as_str()).collect();
    ensure!(keys == ["name", "description"], "{keys:?}");
    ensure!(raw.body == "Body\n");
    let Err(error) = parse_skill_md("no frontmatter") else {
        return Err("a document without frontmatter parsed".into());
    };
    ensure!(error == ParseError::FrontmatterMissing);
    ensure!(error.code() == ErrorCode::FrontmatterMissing);
    Ok(())
}

fn validate_reports_every_error_in_order(tmp: &Path) -> Step {
    let dir = write_raw(
        tmp,
        "dir",
        "---\nzeta: 1\nname: -Bad--Name_\ndescription: d\ncompatibility: ''\nlicense:\n  - a\nmetadata: x\nallowed-tools:\n  - a\n---\n",
    )?;
    let found = codes(&validate(&dir));
    ensure!(
        found
            == [
                ErrorCode::UnexpectedFields,
                ErrorCode::NameNotLowercase,
                ErrorCode::NameHyphenEdge,
                ErrorCode::NameConsecutiveHyphens,
                ErrorCode::NameInvalidChars,
                ErrorCode::NameDirMismatch,
                ErrorCode::CompatibilityEmpty,
                ErrorCode::LicenseNotString,
                ErrorCode::MetadataNotMapping,
                ErrorCode::AllowedToolsNotString,
            ],
        "{found:?}"
    );
    let missing = codes(&validate(&tmp.join("absent")));
    ensure!(missing == [ErrorCode::PathNotFound], "{missing:?}");
    Ok(())
}

fn load_warns_and_keeps_the_skill(tmp: &Path) -> Step {
    let dir = write_raw(
        tmp,
        "warn-dir",
        "---\nname: other-name\ndescription: d\nx-custom: kept\n---\nBody\n",
    )?;
    let loaded = load(&dir)?;
    let found = codes(&loaded.diagnostics);
    ensure!(
        found == [ErrorCode::UnknownField, ErrorCode::NameDirMismatch],
        "{found:?}"
    );
    let keys: Vec<&str> = loaded
        .skill
        .extensions()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    ensure!(keys == ["x-custom"], "{keys:?}");
    ensure!(loaded.skill.location().is_some());
    Ok(())
}

fn load_skips_an_unusable_skill(tmp: &Path) -> Step {
    let dir = write_raw(tmp, "skip-dir", "---\ndescription: d\n---\n")?;
    let Err(skip) = load(&dir) else {
        return Err("a skill without a name loaded".into());
    };
    ensure!(skip.errors[0].code() == ErrorCode::NameMissing);
    ensure!(skip.to_string() == skip.errors[0].message(), "{skip}");
    Ok(())
}

fn builder_writes_a_valid_skill(tmp: &Path) -> Step {
    let skill = Skill::builder("built-skill", "Built through the builder.")
        .license("MIT")
        .metadata("author", "smoke")
        .allowed_tools("Read")
        .body("Do the thing.\n")
        .build()?;
    ensure!(skill.properties.license.as_deref() == Some("MIT"));
    ensure!(skill.properties.metadata["author"] == "smoke");
    ensure!(skill.properties.allowed_tools.as_deref() == Some("Read"));
    let file = skill.write_to(tmp)?;
    let dir = file.parent().ok_or("no parent")?;
    let errors = validate(dir);
    ensure!(errors.is_empty(), "{errors:?}");
    Ok(())
}

fn modify_and_write_round_trips(tmp: &Path) -> Step {
    let dir = write_raw(
        tmp,
        "round-trip",
        "---\nname: round-trip\ndescription: Before.\nx-custom: kept\n---\nBody text.\n",
    )?;
    let mut skill = load(&dir)?.skill;
    skill.properties.description = "After.".to_string();
    skill.write_to(tmp)?;
    let reloaded = load(&dir)?.skill;
    ensure!(reloaded.properties.description == "After.");
    ensure!(reloaded.body == "Body text.\n");
    ensure!(reloaded.extensions() == skill.extensions());
    ensure!(!reloaded.extensions().is_empty());
    Ok(())
}

fn catalog_prompt_omits_empty_and_escapes() -> Step {
    ensure!(to_prompt(&[]).is_none());
    let skill = Skill::builder("escaped", "Tom & <Jerry>").build()?;
    let prompt = to_prompt(&[skill]).ok_or("no prompt for one skill")?;
    ensure!(prompt.contains("Tom &amp; &lt;Jerry&gt;"), "{prompt}");
    ensure!(!prompt.contains("<Jerry>"), "{prompt}");
    Ok(())
}

fn activation_lists_resources_without_reading_them() -> Step {
    let loaded = load(&reference_root().join("webapp-testing"))?;
    let script =
        fs::read_to_string(reference_root().join("webapp-testing/scripts/with_server.py"))?;
    ensure!(script.contains("import subprocess"));
    let text = activation(&loaded.skill);
    ensure!(
        text.contains("<file>scripts/with_server.py</file>"),
        "{text}"
    );
    ensure!(!text.contains("import subprocess"));
    Ok(())
}

fn discover_ranks_and_reports_shadowing(tmp: &Path) -> Step {
    let project = tmp.join("project");
    let user = tmp.join("user");
    write_raw(
        &project,
        "dup",
        "---\nname: dup\ndescription: project copy\n---\n",
    )?;
    write_raw(
        &user,
        "dup",
        "---\nname: dup\ndescription: user copy\n---\n",
    )?;
    write_raw(
        &user,
        "user-only",
        "---\nname: user-only\ndescription: d\n---\n",
    )?;
    let found = discover(&[
        Scope {
            kind: ScopeKind::User,
            root: user,
        },
        Scope {
            kind: ScopeKind::Project,
            root: project,
        },
        Scope {
            kind: ScopeKind::Project,
            root: reference_root(),
        },
    ]);
    let names: Vec<&str> = found
        .skills
        .iter()
        .map(|s| s.properties.name.as_str())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    ensure!(names == sorted, "{names:?}");
    for reference in ["brand-guidelines", "internal-comms", "webapp-testing"] {
        ensure!(names.contains(&reference), "{names:?}");
    }
    let dup = found
        .skills
        .iter()
        .find(|s| s.properties.name.as_str() == "dup")
        .ok_or("dup missing")?;
    ensure!(dup.properties.description == "project copy");
    ensure!(
        codes(found.diagnostics.iter().map(|d| &d.error)) == [ErrorCode::SkillShadowed],
        "{:?}",
        found.diagnostics
    );
    Ok(())
}

fn display_path_is_pathlib_text() -> Step {
    ensure!(display_path(Path::new("./a/b")) == "a/b");
    ensure!(display_path(Path::new("")) == ".");
    Ok(())
}

fn hints() -> ExecutionHints {
    ExecutionHints::new().with_temperature(0.0)
}

async fn llm_picks_and_activates(llm: &dyn LlmClient) -> Step {
    let found = discover(&[Scope {
        kind: ScopeKind::Project,
        root: skills_root(),
    }]);
    let catalog = to_prompt(&found.skills).ok_or("no skills found")?;
    let names: Vec<&str> = found
        .skills
        .iter()
        .map(|s| s.properties.name.as_str())
        .collect();
    let reply = llm
        .complete(
            CompletionRequest::new(format!(
                "{catalog}\n\nTask: Say hello to Ada in a formal tone.\nChoose the one skill that fits the task."
            ))
            .with_execution_hints(hints())
            .with_output_schema(json!({
                "type": "object",
                "properties": {"skill": {"type": "string", "enum": names}},
                "required": ["skill"]
            })),
        )
        .await?;
    let picked = SkillName::parse(
        reply.parsed_output["skill"]
            .as_str()
            .ok_or("no skill picked")?,
    )?;
    let skill = found
        .skills
        .iter()
        .find(|s| s.properties.name == picked)
        .ok_or("picked skill is not in the catalog")?;
    let text = activation(skill);
    ensure!(
        text.contains(&format!("<skill_content name=\"{picked}\">")),
        "{text}"
    );
    Ok(())
}

/// Drafts a skill, building and validating it; feeds library error text back on failure.
async fn llm_drafts_a_valid_skill(
    llm: &dyn LlmClient,
    tmp: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let mut feedback = String::new();
    for _ in 0..3 {
        let draft = llm
            .complete(
                CompletionRequest::new(format!(
                    "Draft an Agent Skill that greets a named person. `name` is lowercase letters, \
                     digits and single hyphens; `description` says what it does and when to use it \
                     (under 1024 characters); `body` is Markdown instructions.{feedback}"
                ))
                .with_execution_hints(hints())
                .with_output_schema(json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "description": {"type": "string"},
                        "body": {"type": "string"}
                    },
                    "required": ["name", "description", "body"]
                })),
            )
            .await?
            .parsed_output;
        let field = |key: &str| draft[key].as_str().ok_or(format!("draft has no `{key}`"));
        let problems: Vec<String> = match SkillBuilder::new(field("name")?, field("description")?)
            .body(field("body")?)
            .build()
        {
            Err(error) => vec![error.to_string()],
            Ok(skill) => {
                let file = skill.write_to(tmp)?;
                let dir = file.parent().ok_or("no parent")?.to_path_buf();
                let errors = validate(&dir);
                if errors.is_empty() {
                    return Ok(dir);
                }
                errors.iter().map(|e| e.to_string()).collect()
            }
        };
        feedback = format!(
            "\nYour previous draft was rejected: {}. Fix it.",
            problems.join("; ")
        );
    }
    Err("no valid draft in 3 attempts".into())
}

async fn llm_revises_the_description(llm: &dyn LlmClient, dir: &Path, tmp: &Path) -> Step {
    let mut skill = load(dir)?.skill;
    let body = skill.body.clone();
    let reply = llm
        .complete(
            CompletionRequest::new(format!(
                "Rewrite this skill description so it also says when to use the skill, in under 1024 characters: {}",
                skill.properties.description
            ))
            .with_execution_hints(hints())
            .with_output_schema(json!({
                "type": "object",
                "properties": {"description": {"type": "string"}},
                "required": ["description"]
            })),
        )
        .await?;
    let revised = reply.parsed_output["description"]
        .as_str()
        .ok_or("no description")?
        .to_string();
    skill.properties.description = revised;
    skill.write_to(tmp)?;
    let errors = validate(dir);
    ensure!(errors.is_empty(), "{errors:?}");
    ensure!(load(dir)?.skill.body == body);
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let tmp = temp.path();

    report("name-parse-trims", name_parse_trims());
    report("name-parse-nfkc", name_parse_nfkc());
    report("name-error-lists-violations", name_error_lists_violations());
    report("parse-skill-md", parse_skill_md_splits_and_rejects());
    report("validate-order", validate_reports_every_error_in_order(tmp));
    report("load-warn", load_warns_and_keeps_the_skill(tmp));
    report("load-skip", load_skips_an_unusable_skill(tmp));
    report("builder-write-validate", builder_writes_a_valid_skill(tmp));
    report("modify-round-trip", modify_and_write_round_trips(tmp));
    report("catalog-prompt", catalog_prompt_omits_empty_and_escapes());
    report(
        "activation-resources",
        activation_lists_resources_without_reading_them(),
    );
    report("discover-scopes", discover_ranks_and_reports_shadowing(tmp));
    report("display-path", display_path_is_pathlib_text());

    let (mode, llm) = client(Stub::new([
        json!({"skill": "greet-user"}),
        json!({
            "name": "greet-person",
            "description": "Greet a named person in a chosen style. Use when asked for a greeting.",
            "body": "# Greet person\n\nWrite one short greeting addressed to the named person.\n"
        }),
        json!({
            "description": "Greet a named person in a chosen style. Use when the user asks for a greeting or a welcome message."
        }),
    ]));
    println!("Mode: {mode}");

    report(
        "llm-pick-and-activate",
        llm_picks_and_activates(llm.as_ref()).await,
    );
    let out = tmp.join("authored");
    match llm_drafts_a_valid_skill(llm.as_ref(), &out).await {
        Ok(dir) => {
            println!("ok llm-draft-write-validate");
            report(
                "llm-revise-description",
                llm_revises_the_description(llm.as_ref(), &dir, &out).await,
            );
        }
        Err(error) => report("llm-draft-write-validate", Err(error)),
    }
    Ok(())
}
