//! Have the model draft a skill's fields, build it through the library, write it,
//! validate it strictly, reload it, edit its description and write it again.
//!
//! `cargo run -p skill-llm --bin author [OUT_DIR]` (default: a temporary directory)

use std::path::PathBuf;

use serde_json::json;
use skill_llm::{Stub, client};
use turul_agent_skills::{Skill, SkillName, SkillProperties, load, validate};
use turul_llm_core::CompletionRequest;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().into());

    let (mode, llm) = client(Stub::new([json!({
        "name": "greet-user",
        "description": "Greet a person by name in a chosen style.",
        "body": "# Greet user\n\nWrite one short greeting addressed to the named person.\n"
    })]));
    println!("Mode: {mode}");

    let draft = llm
        .complete(
            CompletionRequest::new(
                "Draft an Agent Skill that greets a named person. `name` is lowercase letters, \
                 digits and single hyphens; `description` says what it does and when to use it; \
                 `body` is Markdown instructions.",
            )
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

    let name = SkillName::parse(field("name")?).map_err(|e| format!("invalid name: {e:?}"))?;
    let skill = Skill::new(
        SkillProperties::new(name, field("description")?),
        field("body")?,
    );
    let file = skill.write_to(&out)?;
    let dir = file.parent().expect("a written file has a directory");

    let errors = validate(dir);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("{e}");
        }
        std::process::exit(1);
    }

    let mut loaded = load(dir)
        .map_err(|skip| format!("{:?}", skip.errors))?
        .skill;
    loaded
        .properties
        .description
        .push_str(" Use when asked to greet someone.");
    let file = loaded.write_to(&out)?;
    println!("Wrote {}", file.display());
    Ok(())
}
