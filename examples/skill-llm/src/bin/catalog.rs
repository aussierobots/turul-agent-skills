//! Discover skills, show the model the catalog, let it pick one, activate it,
//! then answer with the skill's instructions in front of the model.
//!
//! `cargo run -p skill-llm --bin catalog [SKILLS_DIR]`

use std::path::PathBuf;

use serde_json::json;
use skill_llm::{Stub, client};
use turul_agent_skills::{Scope, ScopeKind, activation, discover, to_prompt};
use turul_llm_core::CompletionRequest;

const TASK: &str = "Say hello to Ada in a formal tone.";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/skills")));
    let found = discover(&[Scope {
        kind: ScopeKind::Project,
        root,
    }]);
    for d in &found.diagnostics {
        eprintln!("{}: {}", d.path.display(), d.error);
    }
    let catalog = to_prompt(&found.skills).ok_or("no skills found")?;
    let names: Vec<&str> = found
        .skills
        .iter()
        .map(|s| s.properties.name.as_str())
        .collect();

    let (mode, llm) = client(Stub::new([
        json!({"skill": "greet-user"}),
        json!({"answer": "Good day, Ada. (offline stub)"}),
    ]));
    println!("Mode: {mode}");

    let pick = llm
        .complete(
            CompletionRequest::new(format!(
                "{catalog}\n\nTask: {TASK}\nChoose the one skill that fits the task."
            ))
            .with_output_schema(json!({
                "type": "object",
                "properties": {"skill": {"type": "string", "enum": names}},
                "required": ["skill"]
            })),
        )
        .await?;
    let picked = pick.parsed_output["skill"]
        .as_str()
        .ok_or("no skill picked")?;
    println!("Picked: {picked}");
    let skill = found
        .skills
        .iter()
        .find(|s| s.properties.name.as_str() == picked)
        .ok_or("picked skill is not in the catalog")?;

    let instructions = activation(skill);
    println!("{instructions}");
    let reply = llm
        .complete(
            CompletionRequest::new(format!("{instructions}\n\nTask: {TASK}")).with_output_schema(
                json!({
                    "type": "object",
                    "properties": {"answer": {"type": "string"}},
                    "required": ["answer"]
                }),
            ),
        )
        .await?;
    println!(
        "Answer: {}",
        reply.parsed_output["answer"].as_str().unwrap_or_default()
    );
    Ok(())
}
