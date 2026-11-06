//! The example binaries run end to end with the offline stub and no network.

use std::path::Path;

use assert_cmd::Command;

fn offline(mut cmd: Command) -> Command {
    cmd.env_remove("OLLAMA_BASE_URL")
        .env_remove("RUN_OLLAMA_SMOKE");
    cmd
}

#[test]
fn catalog_picks_activates_and_answers() {
    let out = offline(assert_cmd::cargo_bin_cmd!("catalog"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Mode: offline-stub"), "{stdout}");
    assert!(stdout.contains("Picked: greet-user"), "{stdout}");
    assert!(
        stdout.contains("<skill_content name=\"greet-user\">"),
        "{stdout}"
    );
    assert!(stdout.contains("Answer: "), "{stdout}");
}

#[test]
fn author_writes_a_valid_skill_and_edits_its_description() {
    let dir = tempfile::tempdir().unwrap();
    let out = offline(assert_cmd::cargo_bin_cmd!("author"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let skill_dir = dir.path().join("greet-user");
    assert!(turul_agent_skills::validate(&skill_dir).is_empty());
    let loaded = turul_agent_skills::load(&skill_dir).unwrap();
    assert!(
        loaded
            .skill
            .properties
            .description
            .ends_with("Use when asked to greet someone.")
    );
    assert!(Path::new(&skill_dir).join("SKILL.md").is_file());
}

#[test]
fn smoke_passes_every_step() {
    let out = offline(assert_cmd::cargo_bin_cmd!("smoke"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Mode: offline-stub"), "{stdout}");
    assert_eq!(
        stdout.matches("\nok ").count() + stdout.starts_with("ok ") as usize,
        18,
        "{stdout}"
    );
    assert!(stdout.contains("ok advise"), "{stdout}");
    assert!(stdout.contains("ok llm-revise-description"), "{stdout}");
    assert!(stdout.contains("ok llm-review-skill"), "{stdout}");
}
