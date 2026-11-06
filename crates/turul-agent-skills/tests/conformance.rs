//! Table-driven conformance over `tests/fixtures/*/expected.json`.

use std::collections::BTreeSet;
use std::fs;

use serde_json::Value;
use tempfile::TempDir;
use turul_agent_skills::{SkillProperties, load, parse_skill_md, to_prompt, validate};

mod common;
use common::{case_path, expected as read_expected, fixture_ids, fixtures_dir, subdirectories};

/// The 40 `skills-ref` 0.1 test cases.
const CORPUS_IDS: [&str; 40] = [
    "p-valid-frontmatter",
    "p-missing-frontmatter",
    "p-unclosed-frontmatter",
    "p-invalid-yaml",
    "p-non-mapping",
    "p-read-valid",
    "p-read-metadata",
    "p-missing-skill-md",
    "p-missing-name",
    "p-missing-description",
    "p-prefer-uppercase",
    "p-accept-lowercase",
    "p-find-none",
    "p-read-lowercase",
    "p-allowed-tools",
    "v-valid",
    "v-nonexistent-path",
    "v-not-a-directory",
    "v-missing-skill-md",
    "v-name-uppercase",
    "v-name-too-long",
    "v-name-leading-hyphen",
    "v-name-double-hyphen",
    "v-name-invalid-chars",
    "v-name-dir-mismatch",
    "v-unexpected-fields",
    "v-all-fields",
    "v-allowed-tools",
    "v-i18n-chinese",
    "v-i18n-russian-hyphen",
    "v-i18n-russian-lower",
    "v-i18n-russian-upper",
    "v-description-too-long",
    "v-compatibility-valid",
    "v-compatibility-too-long",
    "v-nfkc",
    "r-empty",
    "r-single",
    "r-multiple",
    "r-escaping",
];

/// The properties object `read-properties` prints, keys in specification order.
fn properties_object(p: &SkillProperties) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("name".into(), p.name.as_str().into());
    object.insert("description".into(), p.description.as_str().into());
    if let Some(license) = &p.license {
        object.insert("license".into(), license.as_str().into());
    }
    if let Some(compatibility) = &p.compatibility {
        object.insert("compatibility".into(), compatibility.as_str().into());
    }
    if let Some(tools) = &p.allowed_tools {
        object.insert("allowed-tools".into(), tools.as_str().into());
    }
    if !p.metadata.is_empty() {
        let metadata = p
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(v.as_str())))
            .collect();
        object.insert("metadata".into(), Value::Object(metadata));
    }
    Value::Object(object)
}

fn check_parse(id: &str, expected: &Value) {
    let content = fs::read_to_string(fixtures_dir().join(id).join("SKILL.md")).unwrap();
    let result = parse_skill_md(&content);
    let want = expected["parse"].as_str().unwrap();
    match (&result, want) {
        (Ok(_), "ok") => {}
        (Err(e), code) => assert_eq!(e.code().as_str(), code, "{id}"),
        (Ok(_), code) => panic!("{id}: expected {code}, parsed ok"),
    }
    if let Some(entries) = expected.get("entries") {
        let raw = result.unwrap();
        for (key, value) in entries.as_object().unwrap() {
            let found = raw.entries.iter().find(|(k, _)| k == key);
            match found {
                Some((_, turul_agent_skills::RawValue::Str { text, .. })) => {
                    assert_eq!(Some(text.as_str()), value.as_str(), "{id}: {key}");
                }
                other => panic!("{id}: {key}: {other:?}"),
            }
        }
        if let Some(needle) = expected.get("body_contains") {
            assert!(raw.body.contains(needle.as_str().unwrap()), "{id}: body");
        }
    }
}

fn check_directory(id: &str, expected: &Value) {
    let tmp = TempDir::new().unwrap();
    let path = case_path(id, expected, &tmp);

    let errors = validate(&path);
    assert_eq!(
        errors.is_empty(),
        expected["valid"].as_bool().unwrap(),
        "{id}: valid"
    );
    let want: Vec<&str> = expected["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    let got: Vec<&str> = errors.iter().map(|e| e.code().as_str()).collect();
    assert_eq!(got, want, "{id}: errors");

    let loaded = load(&path);
    let outcome = match &loaded {
        Ok(l) if l.diagnostics.is_empty() => "ok",
        Ok(_) => "warn",
        Err(_) => "skip",
    };
    assert_eq!(outcome, expected["load"].as_str().unwrap(), "{id}: load");

    if let Some(properties) = expected.get("properties") {
        let skill = loaded.unwrap_or_else(|_| panic!("{id}: skipped")).skill;
        let got = serde_json::to_string(&properties_object(&skill.properties)).unwrap();
        let want = serde_json::to_string(properties).unwrap();
        assert_eq!(got, want, "{id}: properties");
    }
}

fn check_prompt(id: &str, expected: &Value) {
    let skills: Vec<_> = subdirectories(&fixtures_dir().join(id))
        .iter()
        .map(|dir| {
            load(dir)
                .unwrap_or_else(|_| panic!("{id}: {dir:?} skipped"))
                .skill
        })
        .collect();
    let prompt = to_prompt(&skills);
    let want = &expected["prompt"];
    if want.is_null() {
        assert_eq!(prompt, None, "{id}: prompt");
        return;
    }
    let prompt = prompt.unwrap_or_else(|| panic!("{id}: no prompt"));
    for needle in want["contains"].as_array().unwrap() {
        assert!(
            prompt.contains(needle.as_str().unwrap()),
            "{id}: missing {needle}"
        );
    }
    for needle in want["absent"].as_array().unwrap() {
        assert!(
            !prompt.contains(needle.as_str().unwrap()),
            "{id}: unexpected {needle}"
        );
    }
    for (needle, count) in want["count"].as_object().unwrap() {
        assert_eq!(
            prompt.matches(needle.as_str()).count() as u64,
            count.as_u64().unwrap(),
            "{id}: count of {needle}"
        );
    }
}

#[test]
fn every_corpus_id_has_one_fixture() {
    let ids: BTreeSet<String> = fixture_ids().into_iter().collect();
    for id in CORPUS_IDS {
        assert!(ids.contains(id), "missing fixture {id}");
    }
    let corpus: BTreeSet<&str> = CORPUS_IDS.iter().copied().collect();
    assert_eq!(corpus.len(), 40);
    for id in &ids {
        assert!(
            id.starts_with("house-") || corpus.contains(id.as_str()),
            "unexpected fixture {id}"
        );
    }
}

#[test]
fn corpus_cases_match_expected() {
    let ids = fixture_ids();
    assert!(ids.len() >= 40);
    for id in ids {
        let expected = read_expected(&id);
        if expected.get("parse").is_some() {
            check_parse(&id, &expected);
        }
        if expected.get("valid").is_some() {
            check_directory(&id, &expected);
        }
        if expected.get("prompt").is_some() {
            check_prompt(&id, &expected);
        }
    }
}
