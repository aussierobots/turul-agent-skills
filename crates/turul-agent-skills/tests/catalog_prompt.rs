//! The catalog prompt and the activation text.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use turul_agent_skills::{
    MAX_RESOURCE_FILES, MAX_SCAN_DEPTH, Skill, SkillName, SkillProperties, activation, load,
    to_prompt,
};

mod common;
use common::{fixtures_dir, subdirectories};

/// Writes and loads `<root>/<dir>/SKILL.md` and returns the skill.
fn make_skill(root: &Path, dir: &str, name: &str, description: &str, body: &str) -> Skill {
    let path = root.join(dir);
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: '{}'\n---\n{body}",
            description.replace('\'', "''")
        ),
    )
    .unwrap();
    load(&path)
        .unwrap_or_else(|s| panic!("{:?}", s.errors))
        .skill
}

fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let canonical = tmp.path().canonicalize().unwrap();
    (tmp, canonical)
}

fn bare_skill(name: &str, description: &str) -> Skill {
    Skill::new(
        SkillProperties::new(SkillName::parse(name).unwrap(), description),
        "Body",
    )
}

fn block(name: &str, description: &str, location: &str) -> String {
    format!(
        "<skill>\n<name>\n{name}\n</name>\n<description>\n{description}\n</description>\n<location>\n{location}\n</location>\n</skill>"
    )
}

#[test]
fn to_prompt_empty_is_none() {
    assert_eq!(to_prompt(&[]), None);
}

#[test]
fn to_prompt_one_skill_exact_bytes() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "my-skill", "my-skill", "A test skill", "Body\n");
    let location = format!("{}/my-skill/SKILL.md", root.display());
    assert_eq!(
        to_prompt(&[skill]).unwrap(),
        format!(
            "<available_skills>\n{}\n</available_skills>",
            block("my-skill", "A test skill", &location)
        )
    );
}

#[test]
fn to_prompt_two_skills_exact_bytes() {
    let (_tmp, root) = root();
    let a = make_skill(&root, "skill-a", "skill-a", "First skill", "");
    let b = make_skill(&root, "skill-b", "skill-b", "Second skill", "");
    let prompt = to_prompt(&[a, b]).unwrap();
    assert_eq!(
        prompt,
        format!(
            "<available_skills>\n{}\n{}\n</available_skills>",
            block(
                "skill-a",
                "First skill",
                &format!("{}/skill-a/SKILL.md", root.display())
            ),
            block(
                "skill-b",
                "Second skill",
                &format!("{}/skill-b/SKILL.md", root.display())
            ),
        )
    );
    assert_eq!(prompt.lines().count(), 24);
    assert!(!prompt.ends_with('\n'));
}

#[test]
fn to_prompt_escapes_like_html_escape() {
    let (_tmp, root) = root();
    let skill = make_skill(
        &root,
        "my-skill",
        "my-skill",
        "Use <foo> & <bar> \"q\" 'a' tags",
        "",
    );
    let prompt = to_prompt(&[skill]).unwrap();
    assert!(
        prompt.contains("\nUse &lt;foo&gt; &amp; &lt;bar&gt; &quot;q&quot; &#x27;a&#x27; tags\n"),
        "{prompt}"
    );
    // `&` is escaped first, so entities are not double-escaped.
    assert!(!prompt.contains("&amp;lt;"));
}

#[test]
fn to_prompt_location_not_escaped() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "dir&<x>", "my-skill", "d", "");
    let prompt = to_prompt(&[skill]).unwrap();
    assert!(
        prompt.contains(&format!(
            "<location>\n{}/dir&<x>/SKILL.md\n</location>",
            root.display()
        )),
        "{prompt}"
    );
}

#[test]
fn to_prompt_preserves_input_order() {
    let (_tmp, root) = root();
    let a = make_skill(&root, "a", "a", "da", "");
    let b = make_skill(&root, "b", "b", "db", "");
    let prompt = to_prompt(&[b.clone(), a.clone(), b]).unwrap();
    let names: Vec<&str> = prompt
        .lines()
        .enumerate()
        .filter(|(_, l)| *l == "<name>")
        .map(|(i, _)| prompt.lines().nth(i + 1).unwrap())
        .collect();
    assert_eq!(names, ["b", "a", "b"]);
}

#[test]
fn to_prompt_is_deterministic() {
    let (_tmp, root) = root();
    let skills = [
        make_skill(&root, "a", "a", "da", ""),
        make_skill(&root, "b", "b", "db", ""),
    ];
    assert_eq!(to_prompt(&skills), to_prompt(&skills));
}

#[test]
fn to_prompt_omits_location_without_one() {
    let prompt = to_prompt(&[bare_skill("my-skill", "d")]).unwrap();
    assert_eq!(
        prompt,
        "<available_skills>\n<skill>\n<name>\nmy-skill\n</name>\n<description>\nd\n</description>\n</skill>\n</available_skills>"
    );
}

#[test]
fn to_prompt_matches_corpus_goldens() {
    for id in ["r-single", "r-multiple", "r-escaping"] {
        let case = fixtures_dir().join(id).canonicalize().unwrap();
        let skills: Vec<Skill> = subdirectories(&case)
            .iter()
            .map(|d| load(d).unwrap().skill)
            .collect();
        let golden = fs::read_to_string(case.join("expected.prompt.xml")).unwrap();
        let want = golden.replace("/T", &case.display().to_string());
        assert_eq!(to_prompt(&skills).unwrap(), want, "{id}");
    }
}

#[test]
fn activation_wraps_trimmed_body_with_directory() {
    let (_tmp, root) = root();
    let skill = make_skill(
        &root,
        "my-skill",
        "my-skill",
        "d",
        "\n\n  # Title\n\nText.\n\n",
    );
    assert_eq!(
        activation(&skill),
        format!(
            "<skill_content name=\"my-skill\">\n# Title\n\nText.\n\nSkill directory: {}/my-skill\nRelative paths in this skill are relative to the skill directory.\n</skill_content>",
            root.display()
        )
    );
}

#[test]
fn activation_lists_resources_sorted_without_reading() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "my-skill", "my-skill", "d", "Body");
    let dir = root.join("my-skill");
    for file in [
        "scripts/b.sh",
        "scripts/a.sh",
        "assets/x&y.json",
        "references/doc.md",
        "Z.txt",
    ] {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let secret = dir.join("secret.txt");
        fs::write(&secret, "x").unwrap();
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).unwrap();
    }
    let text = activation(&skill);
    let listed: Vec<&str> = text
        .lines()
        .filter_map(|l| {
            l.strip_prefix("  <file>")
                .and_then(|l| l.strip_suffix("</file>"))
        })
        .collect();
    let mut expected = vec![
        "Z.txt",
        "assets/x&y.json",
        "references/doc.md",
        "scripts/a.sh",
        "scripts/b.sh",
    ];
    if cfg!(unix) {
        expected.insert(2, "secret.txt");
        expected.sort_unstable();
    }
    assert_eq!(listed, expected);
    assert!(!listed.contains(&"SKILL.md"));
    assert!(text.contains("\n\n<skill_resources>\n  <file>Z.txt</file>\n"));
    assert!(text.ends_with("</file>\n</skill_resources>\n</skill_content>"));
}

#[test]
fn activation_omits_resources_block_when_none() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "my-skill", "my-skill", "d", "Body");
    assert!(!activation(&skill).contains("skill_resources"));
    // An empty directory is not a resource.
    fs::create_dir(root.join("my-skill").join("empty")).unwrap();
    assert!(!activation(&skill).contains("skill_resources"));
}

#[test]
fn activation_caps_resource_listing() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "my-skill", "my-skill", "d", "Body");
    let dir = root.join("my-skill");
    assert_eq!(MAX_RESOURCE_FILES, 100);
    for i in 0..(MAX_RESOURCE_FILES + 5) {
        fs::write(dir.join(format!("f{i:03}.txt")), "x").unwrap();
    }
    let text = activation(&skill);
    assert_eq!(text.matches("<file>").count(), MAX_RESOURCE_FILES);
    assert!(
        text.contains(
            "<file>f099.txt</file>\n  (listing truncated at 100 files)\n</skill_resources>"
        )
    );
    assert!(!text.contains("f100.txt"));

    // Exactly at the cap is not truncated.
    fs::remove_file(dir.join("f104.txt")).unwrap();
    for i in 100..104 {
        fs::remove_file(dir.join(format!("f{i:03}.txt"))).unwrap();
    }
    assert!(!activation(&skill).contains("truncated"));
}

#[test]
fn activation_name_attribute_is_the_skill_name() {
    let text = activation(&bare_skill("my-skill", "d"));
    assert!(text.starts_with("<skill_content name=\"my-skill\">\n"));
}

#[test]
fn activation_without_location_omits_directory_and_resources() {
    assert_eq!(
        activation(&Skill::new(
            bare_skill("my-skill", "d").properties,
            "\n Body \n"
        )),
        "<skill_content name=\"my-skill\">\nBody\n</skill_content>"
    );
}

#[cfg(unix)]
#[test]
fn activation_does_not_follow_symlinks_and_caps_depth() {
    let (_tmp, root) = root();
    let skill = make_skill(&root, "my-skill", "my-skill", "d", "Body");
    let dir = root.join("my-skill");

    // A symlinked directory is listed as one file and its contents are not.
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("hidden.txt"), "x").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("link")).unwrap();

    // Directories skipped by name.
    for skipped in [".git", "node_modules"] {
        fs::create_dir_all(dir.join(skipped)).unwrap();
        fs::write(dir.join(skipped).join("x.txt"), "x").unwrap();
    }

    // Files in a directory at MAX_SCAN_DEPTH are listed; one level deeper is not entered.
    let mut deepest = dir.clone();
    for level in 1..=MAX_SCAN_DEPTH {
        deepest = deepest.join(format!("d{level}"));
    }
    fs::create_dir_all(&deepest).unwrap();
    fs::write(deepest.join("at-limit.txt"), "x").unwrap();
    fs::create_dir_all(deepest.join("too-deep")).unwrap();
    fs::write(deepest.join("too-deep").join("beyond.txt"), "x").unwrap();

    let text = activation(&skill);
    assert!(text.contains("  <file>link</file>\n"), "{text}");
    assert!(!text.contains("hidden.txt"));
    assert!(!text.contains(".git") && !text.contains("node_modules"));
    let at_limit = format!(
        "{}at-limit.txt",
        (1..=MAX_SCAN_DEPTH)
            .map(|l| format!("d{l}/"))
            .collect::<String>()
    );
    assert!(text.contains(&format!("<file>{at_limit}</file>")), "{text}");
    assert!(!text.contains("beyond.txt"));
}
