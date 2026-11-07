//! Strict `validate` and lenient `load` over one field check.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use turul_agent_skills::{
    ErrorCode, LoadSkip, Loaded, RawValue, ValidationError, load, load_str, validate,
};

fn write_skill(root: &Path, dir: &str, content: &str) -> PathBuf {
    let path = root.join(dir);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("SKILL.md"), content).unwrap();
    path
}

fn doc(frontmatter: &str) -> String {
    format!("---\n{frontmatter}\n---\nBody text.\n")
}

fn codes(errors: &[ValidationError]) -> Vec<&'static str> {
    errors.iter().map(|e| e.code().as_str()).collect()
}

fn messages(errors: &[ValidationError]) -> Vec<String> {
    errors.iter().map(|e| e.message().to_string()).collect()
}

fn strict(dir: &str, frontmatter: &str) -> Vec<ValidationError> {
    let tmp = TempDir::new().unwrap();
    validate(&write_skill(tmp.path(), dir, &doc(frontmatter)))
}

fn lenient(dir: &str, frontmatter: &str) -> Result<Loaded, LoadSkip> {
    let tmp = TempDir::new().unwrap();
    load(&write_skill(tmp.path(), dir, &doc(frontmatter)))
}

fn loaded(dir: &str, frontmatter: &str) -> Loaded {
    lenient(dir, frontmatter).unwrap_or_else(|skip| panic!("skipped: {:?}", messages(&skip.errors)))
}

const BASE: &str = "name: my-skill\ndescription: A test skill";

#[test]
fn error_codes_are_stable() {
    let codes = [
        (ErrorCode::PathNotFound, "path-not-found"),
        (ErrorCode::NotADirectory, "not-a-directory"),
        (ErrorCode::SkillMdMissing, "skill-md-missing"),
        (ErrorCode::ReadFailed, "read-failed"),
        (ErrorCode::FrontmatterMissing, "frontmatter-missing"),
        (ErrorCode::FrontmatterUnclosed, "frontmatter-unclosed"),
        (ErrorCode::YamlInvalid, "yaml-invalid"),
        (ErrorCode::FrontmatterNotMapping, "frontmatter-not-mapping"),
        (ErrorCode::UnexpectedFields, "unexpected-fields"),
        (ErrorCode::NameMissing, "name-missing"),
        (ErrorCode::NameEmpty, "name-empty"),
        (ErrorCode::NameTooLong, "name-too-long"),
        (ErrorCode::NameNotLowercase, "name-not-lowercase"),
        (ErrorCode::NameHyphenEdge, "name-hyphen-edge"),
        (
            ErrorCode::NameConsecutiveHyphens,
            "name-consecutive-hyphens",
        ),
        (ErrorCode::NameInvalidChars, "name-invalid-chars"),
        (ErrorCode::NameDirMismatch, "name-dir-mismatch"),
        (ErrorCode::DescriptionMissing, "description-missing"),
        (ErrorCode::DescriptionEmpty, "description-empty"),
        (ErrorCode::DescriptionTooLong, "description-too-long"),
        (
            ErrorCode::CompatibilityNotString,
            "compatibility-not-string",
        ),
        (ErrorCode::CompatibilityEmpty, "compatibility-empty"),
        (ErrorCode::CompatibilityTooLong, "compatibility-too-long"),
        (ErrorCode::LicenseNotString, "license-not-string"),
        (ErrorCode::MetadataNotMapping, "metadata-not-mapping"),
        (
            ErrorCode::MetadataValueNotString,
            "metadata-value-not-string",
        ),
        (ErrorCode::AllowedToolsNotString, "allowed-tools-not-string"),
        (ErrorCode::YamlRepaired, "yaml-repaired"),
        (ErrorCode::UnknownField, "unknown-field"),
    ];
    for (code, text) in codes {
        assert_eq!(code.as_str(), text);
    }
    let unique: BTreeSet<&str> = codes.iter().map(|(_, text)| *text).collect();
    assert_eq!(unique.len(), codes.len(), "codes are unique");
}

#[test]
fn messages_match_reference_text() {
    let long_name = "a".repeat(65);
    let cases: Vec<(&str, String, String)> = vec![
        ("my-skill", "name: my-skill\ndescription: d\nzeta: 1\nalpha: 2".into(),
         "Unexpected fields in frontmatter: alpha, zeta. Only ['allowed-tools', 'compatibility', 'description', 'license', 'metadata', 'name'] are allowed.".into()),
        ("my-skill", "description: d".into(), "Missing required field in frontmatter: name".into()),
        ("my-skill", "name: ''\ndescription: d".into(), "Field 'name' must be a non-empty string".into()),
        ("my-skill", "name:\n  - a\ndescription: d".into(), "Field 'name' must be a non-empty string".into()),
        ("my-skill", "name: '   '\ndescription: d".into(), "Field 'name' must be a non-empty string".into()),
        ("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", format!("name: {long_name}\ndescription: d"),
         format!("Skill name '{long_name}' exceeds 64 character limit (65 chars)")),
        ("MySkill", "name: MySkill\ndescription: d".into(), "Skill name 'MySkill' must be lowercase".into()),
        ("-my-skill", "name: -my-skill\ndescription: d".into(), "Skill name cannot start or end with a hyphen".into()),
        ("my--skill", "name: my--skill\ndescription: d".into(), "Skill name cannot contain consecutive hyphens".into()),
        ("my_skill", "name: my_skill\ndescription: d".into(),
         "Skill name 'my_skill' contains invalid characters. Only letters, digits, and hyphens are allowed.".into()),
        ("wrong-name", "name: correct-name\ndescription: d".into(), "Directory name 'wrong-name' must match skill name 'correct-name'".into()),
        ("my-skill", "name: my-skill".into(), "Missing required field in frontmatter: description".into()),
        ("my-skill", "name: my-skill\ndescription: ''".into(), "Field 'description' must be a non-empty string".into()),
        ("my-skill", "name: my-skill\ndescription: '  '".into(), "Field 'description' must be a non-empty string".into()),
        ("my-skill", "name: my-skill\ndescription:\n  a: b".into(), "Field 'description' must be a non-empty string".into()),
        ("my-skill", format!("name: my-skill\ndescription: {}", "x".repeat(1100)), "Description exceeds 1024 character limit (1100 chars)".into()),
        ("my-skill", format!("{BASE}\ncompatibility:\n  - a"), "Field 'compatibility' must be a string".into()),
        ("my-skill", format!("{BASE}\ncompatibility: ''"), "Field 'compatibility' must be a non-empty string".into()),
        ("my-skill", format!("{BASE}\ncompatibility: {}", "x".repeat(550)), "Compatibility exceeds 500 character limit (550 chars)".into()),
        ("my-skill", format!("{BASE}\nlicense:\n  - MIT"), "Field 'license' must be a string".into()),
        ("my-skill", format!("{BASE}\nmetadata: text"), "Field 'metadata' must be a mapping of string keys to string values".into()),
        ("my-skill", format!("{BASE}\nmetadata:\n  tags:\n    - a"), "Field 'metadata' value for key 'tags' must be a string".into()),
        ("my-skill", format!("{BASE}\nallowed-tools:\n  - Bash"), "Field 'allowed-tools' must be a string".into()),
    ];
    for (dir, frontmatter, expected) in cases {
        let errors = strict(dir, &frontmatter);
        assert_eq!(
            messages(&errors),
            std::slice::from_ref(&expected),
            "{frontmatter}"
        );
        assert_eq!(errors[0].to_string(), expected, "Display is the message");
    }
}

#[test]
fn path_and_parse_messages_match_reference_text() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let missing = root.join("nonexistent");
    assert_eq!(
        messages(&validate(&missing)),
        [format!("Path does not exist: {}", missing.display())]
    );

    let file = root.join("file.txt");
    fs::write(&file, "test").unwrap();
    assert_eq!(
        messages(&validate(&file)),
        [format!("Not a directory: {}", file.display())]
    );

    let empty = root.join("empty");
    fs::create_dir(&empty).unwrap();
    assert_eq!(
        messages(&validate(&empty)),
        ["Missing required file: SKILL.md"]
    );
    let skip = load(&empty).unwrap_err();
    assert_eq!(codes(&skip.errors), ["skill-md-missing"]);
    assert_eq!(
        messages(&skip.errors),
        [format!("SKILL.md not found in {}", empty.display())]
    );

    let parse_cases = [
        (
            "# No frontmatter\n",
            "SKILL.md must start with YAML frontmatter (---)",
        ),
        (
            "---\nname: a\n",
            "SKILL.md frontmatter not properly closed with ---",
        ),
        (
            "---\n- a\n---\n",
            "SKILL.md frontmatter must be a YAML mapping",
        ),
    ];
    for (content, expected) in parse_cases {
        let dir = write_skill(root, "parse-case", content);
        assert_eq!(messages(&validate(&dir)), [expected], "{content}");
    }
    let dir = write_skill(root, "parse-case", "---\nname: [invalid\n---\n");
    let errors = validate(&dir);
    assert_eq!(codes(&errors), ["yaml-invalid"]);
    assert!(
        errors[0]
            .message()
            .starts_with("Invalid YAML in frontmatter: "),
        "{}",
        errors[0]
    );
}

#[test]
fn validate_stops_at_first_path_or_parse_error() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "my-skill", "---\nunknown: 1\n- broken\n");
    assert_eq!(validate(&dir).len(), 1);
    let dir = write_skill(tmp.path(), "other", "no frontmatter and no fields");
    assert_eq!(codes(&validate(&dir)), ["frontmatter-missing"]);
}

#[test]
fn validate_reports_all_errors_in_reference_order() {
    let long = "x".repeat(1100);
    let frontmatter = format!(
        "zeta: 1\nname: -Bad--Name_\ndescription: {long}\ncompatibility: ''\nlicense:\n  - a\nmetadata: x\nallowed-tools:\n  - a"
    );
    assert_eq!(
        codes(&strict("dir", &frontmatter)),
        [
            "unexpected-fields",
            "name-not-lowercase",
            "name-hyphen-edge",
            "name-consecutive-hyphens",
            "name-invalid-chars",
            "name-dir-mismatch",
            "description-too-long",
            "compatibility-empty",
            "license-not-string",
            "metadata-not-mapping",
            "allowed-tools-not-string",
        ]
    );
    let long_name = "a".repeat(65);
    assert_eq!(
        codes(&strict(
            "dir",
            &format!("name: {long_name}\ndescription: d")
        )),
        ["name-too-long", "name-dir-mismatch"]
    );
}

#[test]
fn unknown_key_is_strict_error_single_message_sorted() {
    let errors = strict("my-skill", &format!("{BASE}\nzz: 1\naa: 2"));
    assert_eq!(codes(&errors), ["unexpected-fields"]);
    assert!(
        errors[0]
            .message()
            .starts_with("Unexpected fields in frontmatter: aa, zz. ")
    );
}

#[test]
fn unknown_key_is_lenient_diagnostic_per_key_and_kept_in_extensions() {
    let loaded = loaded(
        "my-skill",
        &format!("zz: 1\n{BASE}\ndisable-model-invocation: true\nlist:\n  - a"),
    );
    assert_eq!(
        codes(&loaded.diagnostics),
        ["unknown-field", "unknown-field", "unknown-field"]
    );
    assert_eq!(
        messages(&loaded.diagnostics)[0],
        "Unknown frontmatter field 'zz' kept as an extension"
    );
    let extensions = loaded.skill.extensions();
    let keys: Vec<&str> = extensions.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["zz", "disable-model-invocation", "list"]);
    assert_eq!(
        extensions[0].1,
        RawValue::Str {
            text: "1".into(),
            typed: true
        }
    );
    assert_eq!(
        extensions[1].1,
        RawValue::Str {
            text: "true".into(),
            typed: true
        }
    );
    assert!(matches!(extensions[2].1, RawValue::Seq(_)));
}

#[test]
fn lowercase_skill_md_accepted_uppercase_preferred() {
    let tmp = TempDir::new().unwrap();
    let only_lower = tmp.path().join("my-skill");
    fs::create_dir(&only_lower).unwrap();
    fs::write(only_lower.join("skill.md"), doc(BASE)).unwrap();
    assert!(validate(&only_lower).is_empty());
    let skill = load(&only_lower).unwrap().skill;
    assert!(skill.location().unwrap().ends_with("skill.md"));

    let both = tmp.path().join("both");
    fs::create_dir(&both).unwrap();
    fs::write(both.join("SKILL.md"), doc("name: both\ndescription: upper")).unwrap();
    fs::write(
        both.join("lower-copy.md"),
        doc("name: both\ndescription: lower"),
    )
    .unwrap();
    // A case-insensitive file system cannot hold both names; compare only where it can.
    if fs::rename(both.join("lower-copy.md"), both.join("skill.md")).is_ok()
        && fs::read_dir(&both).unwrap().count() == 2
    {
        let skill = load(&both).unwrap().skill;
        assert_eq!(skill.properties.description, "upper");
        assert!(skill.location().unwrap().ends_with("SKILL.md"));
    }
}

#[test]
fn name_too_long_strict_error_lenient_diagnostic() {
    let name = "a".repeat(65);
    let frontmatter = format!("name: {name}\ndescription: d");
    assert_eq!(codes(&strict(&name, &frontmatter)), ["name-too-long"]);
    let loaded = loaded(&name, &frontmatter);
    assert_eq!(codes(&loaded.diagnostics), ["name-too-long"]);
    assert_eq!(loaded.skill.properties.name.as_str(), name);
}

#[test]
fn dir_mismatch_strict_error_lenient_diagnostic() {
    let frontmatter = "name: right-name\ndescription: d";
    assert_eq!(
        codes(&strict("wrong-name", frontmatter)),
        ["name-dir-mismatch"]
    );
    let loaded = loaded("wrong-name", frontmatter);
    assert_eq!(codes(&loaded.diagnostics), ["name-dir-mismatch"]);
    assert_eq!(loaded.skill.properties.name.as_str(), "right-name");
}

#[test]
fn dir_match_uses_nfkc() {
    // Directory is U+00E9, the name is `e` + U+0301.
    let frontmatter = "name: cafe\u{301}\ndescription: d";
    assert!(strict("caf\u{e9}", frontmatter).is_empty());
    let loaded = loaded("caf\u{e9}", frontmatter);
    assert!(loaded.diagnostics.is_empty());
    assert_eq!(loaded.skill.properties.name.as_str(), "caf\u{e9}");
}

#[test]
fn missing_description_skips() {
    for frontmatter in [
        "name: my-skill",
        "name: my-skill\ndescription: ''",
        "name: my-skill\ndescription: '  '",
    ] {
        let skip = lenient("my-skill", frontmatter).unwrap_err();
        assert!(
            skip.errors[0].code() == ErrorCode::DescriptionMissing
                || skip.errors[0].code() == ErrorCode::DescriptionEmpty
        );
    }
}

#[test]
fn unparseable_yaml_skips() {
    let skip = lenient("my-skill", "name: [invalid").unwrap_err();
    assert_eq!(codes(&skip.errors), ["yaml-invalid"]);
}

#[test]
fn repaired_yaml_strict_error_lenient_diagnostic() {
    let frontmatter = "name: my-skill\ndescription: Use when: the user asks";
    let errors = strict("my-skill", frontmatter);
    assert_eq!(codes(&errors), ["yaml-invalid"]);
    let loaded = loaded("my-skill", frontmatter);
    assert_eq!(codes(&loaded.diagnostics), ["yaml-repaired"]);
    assert!(
        loaded.diagnostics[0]
            .message()
            .starts_with("Frontmatter YAML was repaired by quoting values that contain ': ' (")
    );
    assert_eq!(
        loaded.skill.properties.description,
        "Use when: the user asks"
    );
}

#[test]
fn invalid_name_skips() {
    for (dir, name) in [
        ("MySkill", "MySkill"),
        ("-a", "-a"),
        ("a--b", "a--b"),
        ("a_b", "a_b"),
    ] {
        let skip = lenient(dir, &format!("name: {name}\ndescription: d")).unwrap_err();
        assert!(!skip.errors.is_empty(), "{name}");
    }
    let skip = lenient("a_b", "name: a_b\ndescription: d").unwrap_err();
    assert_eq!(codes(&skip.errors), ["name-invalid-chars"]);
    let skip = lenient("x", "description: d").unwrap_err();
    assert_eq!(codes(&skip.errors), ["name-missing"]);
}

#[test]
fn skip_carries_every_error_with_the_skip_cause_first() {
    let skip = lenient("dir", "name: Bad_Name\ndescription: ''\nzz: 1").unwrap_err();
    assert_eq!(
        codes(&skip.errors),
        [
            "description-empty",
            "name-not-lowercase",
            "name-invalid-chars",
            "name-dir-mismatch"
        ]
    );
}

#[test]
fn non_string_optional_fields_strict_error_lenient_drop() {
    let frontmatter = format!(
        "{BASE}\nlicense:\n  - a\ncompatibility:\n  - a\nallowed-tools:\n  - a\nmetadata:\n  ok: fine\n  bad:\n    - x"
    );
    assert_eq!(
        codes(&strict("my-skill", &frontmatter)),
        [
            "compatibility-not-string",
            "license-not-string",
            "metadata-value-not-string",
            "allowed-tools-not-string"
        ]
    );
    let loaded = loaded("my-skill", &frontmatter);
    assert_eq!(
        codes(&loaded.diagnostics),
        [
            "compatibility-not-string",
            "license-not-string",
            "metadata-value-not-string",
            "allowed-tools-not-string"
        ]
    );
    let properties = &loaded.skill.properties;
    assert_eq!(properties.license, None);
    assert_eq!(properties.compatibility, None);
    assert_eq!(properties.allowed_tools, None);
    assert_eq!(properties.metadata.len(), 1);
    assert_eq!(properties.metadata["ok"], "fine");

    let loaded = self::loaded("my-skill", &format!("{BASE}\nmetadata: text"));
    assert_eq!(codes(&loaded.diagnostics), ["metadata-not-mapping"]);
    assert!(loaded.skill.properties.metadata.is_empty());
}

#[test]
fn optional_fields_load_with_stored_values() {
    let frontmatter = String::from(
        "name: my-skill\ndescription: '  padded  '\nlicense: MIT\ncompatibility: Python 3\nallowed-tools: Bash(jq:*) Bash(git:*)\nmetadata:\n  b: 2\n  a: x\n  version: 1.0",
    );
    let loaded = loaded("my-skill", &frontmatter);
    let p = &loaded.skill.properties;
    assert!(loaded.diagnostics.is_empty());
    assert_eq!(p.description, "padded");
    assert_eq!(p.license.as_deref(), Some("MIT"));
    assert_eq!(p.compatibility.as_deref(), Some("Python 3"));
    assert_eq!(p.allowed_tools.as_deref(), Some("Bash(jq:*) Bash(git:*)"));
    let metadata: Vec<(&str, &str)> = p
        .metadata
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert_eq!(metadata, [("a", "x"), ("b", "2"), ("version", "1.0")]);
    assert_eq!(loaded.skill.body, "Body text.\n");
    // Strict validation sees the same file as valid.
    assert!(strict("my-skill", &frontmatter).is_empty());
}

#[test]
fn empty_metadata_value_is_an_empty_mapping() {
    assert!(strict("my-skill", &format!("{BASE}\nmetadata:")).is_empty());
    assert!(
        loaded("my-skill", &format!("{BASE}\nmetadata:"))
            .skill
            .properties
            .metadata
            .is_empty()
    );
}

#[test]
fn compatibility_empty_rejected() {
    assert_eq!(
        codes(&strict("my-skill", &format!("{BASE}\ncompatibility: ''"))),
        ["compatibility-empty"]
    );
    let loaded = loaded("my-skill", &format!("{BASE}\ncompatibility: ''"));
    assert_eq!(codes(&loaded.diagnostics), ["compatibility-empty"]);
    assert_eq!(loaded.skill.properties.compatibility, None);
    let too_long = format!("{BASE}\ncompatibility: {}", "x".repeat(501));
    let loaded = self::loaded("my-skill", &too_long);
    assert_eq!(codes(&loaded.diagnostics), ["compatibility-too-long"]);
    assert_eq!(
        loaded
            .skill
            .properties
            .compatibility
            .as_ref()
            .map(String::len),
        Some(501)
    );
}

#[test]
fn read_failed_on_non_utf8_skill_md() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("my-skill");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), [0xff, 0xfe, 0x00]).unwrap();
    let errors = validate(&dir);
    assert_eq!(codes(&errors), ["read-failed"]);
    assert!(
        errors[0].message().starts_with("Failed to read "),
        "{}",
        errors[0]
    );
    assert!(errors[0].message().contains("SKILL.md"));
    assert_eq!(codes(&load(&dir).unwrap_err().errors), ["read-failed"]);
}

#[cfg(unix)]
#[test]
fn load_location_is_canonical_absolute() {
    let tmp = TempDir::new().unwrap();
    let real = write_skill(tmp.path(), "my-skill", &doc(BASE));
    let link = tmp.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let location = load(&link).unwrap().skill.location().unwrap().to_path_buf();
    assert!(location.is_absolute());
    assert_eq!(location, real.canonicalize().unwrap().join("SKILL.md"));
}

#[test]
fn validate_and_load_never_panic() {
    let pieces = [
        "---\n",
        "name: a\n",
        "description: d\n",
        "name:\n",
        "metadata:\n  k: v\n",
        "  - x\n",
        "[",
        "{",
        ": ",
        "\u{0}",
        "é\n",
        "---",
        "\r\n",
        "&a ",
        "license: \n",
        "compatibility: 1\n",
        "zz: 1\n",
    ];
    let tmp = TempDir::new().unwrap();
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    for round in 0..300 {
        let mut content = String::new();
        for _ in 0..(state % 10) {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            content.push_str(pieces[(state >> 33) as usize % pieces.len()]);
        }
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let dir = write_skill(tmp.path(), &format!("d{}", round % 3), &content);
        let _ = validate(&dir);
        let _ = load(&dir);
    }
    let _ = validate(Path::new(""));
    let _ = load(Path::new(""));
}

#[test]
fn load_skip_displays_first_error() {
    let skip = lenient("my-skill", "description: d").unwrap_err();
    assert_eq!(
        skip.to_string(),
        "Missing required field in frontmatter: name"
    );
    assert_eq!(skip.to_string(), skip.errors[0].message());
    let source: &dyn std::error::Error = &skip;
    assert_eq!(source.to_string(), skip.to_string());
}

#[test]
fn error_code_displays_its_stable_code() {
    assert_eq!(ErrorCode::PathNotFound.to_string(), "path-not-found");
    assert_eq!(ErrorCode::NameDirMismatch.to_string(), "name-dir-mismatch");
}

#[test]
fn load_str_agrees_with_load_on_every_fixture() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs = Vec::new();
    for case in fs::read_dir(&fixtures).unwrap() {
        let case = case.unwrap().path();
        dirs.push(case.clone());
        if case.is_dir() {
            dirs.extend(fs::read_dir(&case).unwrap().map(|e| e.unwrap().path()));
        }
    }
    let mut compared = 0;
    for dir in dirs {
        let Some(file) = ["SKILL.md", "skill.md"]
            .iter()
            .map(|name| dir.join(name))
            .find(|file| file.is_file())
        else {
            continue;
        };
        let name = dir.file_name().unwrap().to_str().unwrap();
        let from_text = load_str(name, &fs::read_to_string(&file).unwrap());
        match (load(&dir), from_text) {
            (Ok(disk), Ok(text)) => {
                assert_eq!(disk.skill, text.skill, "{name}");
                assert_eq!(disk.diagnostics, text.diagnostics, "{name}");
                assert!(disk.skill.location().is_some(), "{name}");
                assert!(text.skill.location().is_none(), "{name}");
            }
            (Err(disk), Err(text)) => assert_eq!(disk, text, "{name}"),
            (disk, text) => panic!("{name}: load {disk:?} but load_str {text:?}"),
        }
        compared += 1;
    }
    assert!(compared >= 40, "compared only {compared} fixtures");
}

#[test]
fn load_str_checks_name_against_the_given_dir_name() {
    let text = doc("name: pdf-tools\ndescription: Work with PDFs.");
    assert!(load_str("pdf-tools", &text).unwrap().diagnostics.is_empty());
    let mismatch = load_str("other", &text).unwrap();
    assert_eq!(codes(&mismatch.diagnostics), ["name-dir-mismatch"]);
}

#[test]
fn load_str_reports_parse_errors_as_load() {
    let skip = load_str("x", "no frontmatter").unwrap_err();
    assert_eq!(skip.errors[0].code(), ErrorCode::FrontmatterMissing);
}
