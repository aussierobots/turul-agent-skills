//! The `turul-skills` command line against the `skills-ref` command contract.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
use turul_agent_skills::{load, to_prompt};

#[path = "../../turul-agent-skills/tests/common/mod.rs"]
mod common;
use common::{case_path, expected, fixture_ids, fixtures_dir};

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(assert_cmd::cargo::cargo_bin!("turul-skills"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

fn write_skill(root: &Path, dir: &str, file: &str, content: &str) {
    fs::create_dir_all(root.join(dir)).unwrap();
    fs::write(root.join(dir).join(file), content).unwrap();
}

const VALID: &str = "---\nname: my-skill\ndescription: A test skill\n---\nBody\n";

#[test]
fn binary_name_is_turul_skills() {
    let tmp = TempDir::new().unwrap();
    let r = run(tmp.path(), &["--help"]);
    assert_eq!(r.code, 0);
    assert!(r.stdout.contains("turul-skills"), "{}", r.stdout);
}

#[test]
fn validate_valid_exit_0_stdout() {
    let tmp = TempDir::new().unwrap();
    write_skill(tmp.path(), "my-skill", "SKILL.md", VALID);
    let r = run(tmp.path(), &["validate", "my-skill"]);
    assert_eq!(
        (r.code, r.stdout.as_str(), r.stderr.as_str()),
        (0, "Valid skill: my-skill\n", "")
    );
}

#[test]
fn validate_skill_md_path_uses_parent() {
    let tmp = TempDir::new().unwrap();
    write_skill(tmp.path(), "my-skill", "SKILL.md", VALID);
    write_skill(
        tmp.path(),
        "other",
        "skill.md",
        &VALID.replace("my-skill", "other"),
    );
    let r = run(tmp.path(), &["validate", "my-skill/SKILL.md"]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "Valid skill: my-skill\n"));
    let r = run(tmp.path(), &["validate", "other/skill.md"]);
    assert_eq!((r.code, r.stdout.as_str()), (0, "Valid skill: other\n"));
}

#[test]
fn validate_errors_exit_1_stderr_lines() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "my-skill",
        "SKILL.md",
        "---\nname: my-skill\ndescription: d\nextra: 1\n---\n",
    );
    let r = run(tmp.path(), &["validate", "my-skill"]);
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "");
    assert!(
        r.stderr
            .starts_with("Validation failed for my-skill:\n  - "),
        "{}",
        r.stderr
    );
    assert!(r.stderr.ends_with('\n'));
}

#[test]
fn validate_regular_file_not_a_directory() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("file.txt"), "test").unwrap();
    let r = run(tmp.path(), &["validate", "./file.txt"]);
    assert_eq!(r.code, 1);
    assert_eq!(
        r.stderr,
        "Validation failed for file.txt:\n  - Not a directory: file.txt\n"
    );
}

#[test]
fn read_properties_json_bytes() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "my-skill",
        "SKILL.md",
        "---\nname: my-skill\ndescription: A test skill\nlicense: MIT\ncompatibility: c\nallowed-tools: Bash\nmetadata:\n  author: Test Author\n  version: 1.0\n---\n",
    );
    let r = run(tmp.path(), &["read-properties", "my-skill"]);
    assert_eq!(r.code, 0);
    assert_eq!(
        r.stdout,
        "{\n  \"name\": \"my-skill\",\n  \"description\": \"A test skill\",\n  \"license\": \"MIT\",\n  \"compatibility\": \"c\",\n  \"allowed-tools\": \"Bash\",\n  \"metadata\": {\n    \"author\": \"Test Author\",\n    \"version\": \"1.0\"\n  }\n}\n"
    );
}

#[test]
fn read_properties_skip_exit_1() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "my-skill",
        "SKILL.md",
        "---\nname: my-skill\n---\n",
    );
    let r = run(tmp.path(), &["read-properties", "my-skill"]);
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "");
    assert!(r.stderr.starts_with("Error: "), "{}", r.stderr);
    assert_eq!(r.stderr.lines().count(), 1);
}

#[test]
fn repaired_yaml_exit_1_like_skills_ref() {
    let tmp = TempDir::new().unwrap();
    let skill = fixtures_dir().join("house-colon-in-value/my-skill");
    let skill = skill.to_str().unwrap();
    for args in [["read-properties", skill], ["to-prompt", skill]] {
        let r = run(tmp.path(), &args);
        assert_eq!(r.code, 1, "{}", r.stdout);
        assert_eq!(r.stdout, "");
        assert!(
            r.stderr.starts_with("Error: Invalid YAML in frontmatter: "),
            "{}",
            r.stderr
        );
        assert_eq!(r.stderr.lines().count(), 1);
    }
}

#[test]
fn read_properties_json_ensure_ascii() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "my-skill",
        "SKILL.md",
        "---\nname: my-skill\ndescription: \"q\\\" b\\\\ n\\n t\\t c\\x01 é 😀 del\\x7f\"\n---\n",
    );
    let r = run(tmp.path(), &["read-properties", "my-skill"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(
        r.stdout,
        "{\n  \"name\": \"my-skill\",\n  \"description\": \"q\\\" b\\\\ n\\n t\\t c\\u0001 \\u00e9 \\ud83d\\ude00 del\x7f\"\n}\n"
    );
}

#[test]
fn printed_name_is_nfkc() {
    let tmp = TempDir::new().unwrap();
    let skill = fixtures_dir().join("v-nfkc");
    let dir = fs::read_dir(&skill)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let dir = dir.to_str().unwrap();
    let r = run(tmp.path(), &["read-properties", dir]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(
        r.stdout.contains("\"name\": \"caf\\u00e9\""),
        "{}",
        r.stdout
    );
    let r = run(tmp.path(), &["to-prompt", dir]);
    assert!(r.stdout.contains("\ncafé\n"), "{}", r.stdout);
}

#[test]
fn read_properties_metadata_sorted_when_source_unsorted() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "my-skill",
        "SKILL.md",
        "---\nname: my-skill\ndescription: d\nmetadata:\n  version: '1'\n  author: a\n---\n",
    );
    let r = run(tmp.path(), &["read-properties", "my-skill"]);
    let author = r.stdout.find("author").unwrap();
    assert!(author < r.stdout.find("version").unwrap(), "{}", r.stdout);
}

#[test]
fn to_prompt_bytes_match_library() {
    let tmp = TempDir::new().unwrap();
    let a = fixtures_dir().join("r-multiple/skill-a");
    let b = fixtures_dir().join("r-multiple/skill-b");
    let r = run(
        tmp.path(),
        &["to-prompt", a.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    let skills: Vec<_> = [&a, &b].iter().map(|d| load(d).unwrap().skill).collect();
    assert_eq!(r.stdout, format!("{}\n", to_prompt(&skills).unwrap()));
}

#[test]
fn to_prompt_skip_exit_1_absolute_path() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "good",
        "SKILL.md",
        &VALID.replace("my-skill", "good"),
    );
    fs::create_dir(tmp.path().join("empty")).unwrap();
    let r = run(tmp.path(), &["to-prompt", "good", "empty"]);
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout, "");
    let root = tmp.path().canonicalize().unwrap();
    assert_eq!(
        r.stderr,
        format!("Error: SKILL.md not found in {}/empty\n", root.display())
    );
}

#[test]
fn missing_path_exit_2() {
    let tmp = TempDir::new().unwrap();
    for (cmd, arg) in [
        ("validate", "SKILL_PATH"),
        ("read-properties", "SKILL_PATH"),
        ("to-prompt", "SKILL_PATHS"),
    ] {
        let r = run(tmp.path(), &[cmd, "nope"]);
        assert_eq!(r.code, 2, "{cmd}");
        assert_eq!(r.stdout, "");
        assert_eq!(
            r.stderr.lines().last().unwrap(),
            format!("Error: Invalid value for '{arg}': Path 'nope' does not exist.")
        );
    }
}

#[test]
fn to_prompt_without_paths_exit_2() {
    let tmp = TempDir::new().unwrap();
    let r = run(tmp.path(), &["to-prompt"]);
    assert_eq!(r.code, 2);
    assert_eq!(r.stdout, "");
    assert!(r.stderr.contains("SKILL_PATHS"), "{}", r.stderr);
}

#[test]
fn paths_print_like_pathlib() {
    let tmp = TempDir::new().unwrap();
    write_skill(tmp.path(), "x", "SKILL.md", &VALID.replace("my-skill", "x"));
    let r = run(tmp.path(), &["validate", "./x/"]);
    assert_eq!(r.stdout, "Valid skill: x\n");
    let r = run(&tmp.path().join("x"), &["validate", "SKILL.md"]);
    assert!(
        r.stderr.starts_with("Validation failed for .:\n"),
        "{}",
        r.stderr
    );
}

#[test]
fn cli_matches_table_on_every_fixture() {
    let tmp_cwd = TempDir::new().unwrap();
    for id in fixture_ids() {
        let want = expected(&id);
        if want.get("valid").is_none() {
            continue;
        }
        let tmp = TempDir::new().unwrap();
        let path = case_path(&id, &want, &tmp);
        let arg = path.to_str().unwrap();
        let missing = want["layout"] == "missing";
        let codes = want["errors"].as_array().unwrap().len();

        let v = run(tmp_cwd.path(), &["validate", arg]);
        let p = run(tmp_cwd.path(), &["read-properties", arg]);
        let t = run(tmp_cwd.path(), &["to-prompt", arg]);
        if missing {
            assert_eq!((v.code, p.code, t.code), (2, 2, 2), "{id}");
            continue;
        }
        if want["valid"] == Value::Bool(true) {
            assert_eq!((v.code, v.stderr.as_str()), (0, ""), "{id}");
            assert!(v.stdout.starts_with("Valid skill: "), "{id}");
        } else {
            assert_eq!(v.code, 1, "{id}");
            assert_eq!(v.stderr.lines().count(), 1 + codes, "{id}: {}", v.stderr);
        }
        // A document that loads only through the colon repair is a YAML error here.
        let repaired = want["errors"]
            .as_array()
            .unwrap()
            .contains(&Value::from("yaml-invalid"));
        let loads = want["load"] != "skip" && !repaired;
        assert_eq!(p.code, i32::from(!loads), "{id}: read-properties");
        assert_eq!(t.code, i32::from(!loads), "{id}: to-prompt");
        if !loads {
            assert!(p.stderr.starts_with("Error: "), "{id}");
            assert_eq!(p.stderr, t.stderr, "{id}");
        }
    }
}

#[test]
fn skip_reason_is_the_error_that_caused_the_skip() {
    let tmp = TempDir::new().unwrap();
    write_skill(tmp.path(), "foo", "SKILL.md", "---\nname: bar\n---\nBody\n");
    let r = run(tmp.path(), &["read-properties", "foo"]);
    assert_eq!(
        (r.code, r.stderr.as_str()),
        (
            1,
            "Error: Missing required field in frontmatter: description\n"
        )
    );
}
