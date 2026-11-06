//! Canonical `SKILL.md` rendering and `write_to`.

use std::fs;

use tempfile::TempDir;
use turul_agent_skills::{Skill, load, validate};

mod common;
use common::{case_path, expected as read_expected, fixture_ids, fixtures_dir};

fn skill(description: &str) -> Skill {
    Skill::builder("my-skill", description)
        .body("Body\n")
        .build()
        .unwrap()
}

/// The rendered form of `value` as a metadata value (the builder trims descriptions).
fn scalar_line(value: &str) -> String {
    let mut skill = skill("d");
    skill.properties.metadata.insert("k".into(), value.into());
    let rendered = skill.render();
    let line = rendered
        .lines()
        .find(|line| line.starts_with("  k:"))
        .unwrap_or_else(|| panic!("{rendered:?}"));
    line.strip_prefix("  k:")
        .unwrap()
        .trim_start_matches(' ')
        .to_string()
}

/// Writes `content` as `<name>/SKILL.md` in a fresh directory and loads it.
fn load_text(name: &str, content: &str) -> (TempDir, Skill) {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join(name);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), content).unwrap();
    let skill = load(&dir)
        .unwrap_or_else(|s| panic!("{:?}", s.errors))
        .skill;
    (tmp, skill)
}

/// Fixtures that load, with their case path kept alive by the returned guard.
fn loadable_fixtures() -> Vec<(String, TempDir, Skill)> {
    let mut found = Vec::new();
    for id in fixture_ids() {
        let expected = read_expected(&id);
        if expected.get("valid").is_none() {
            continue;
        }
        let tmp = TempDir::new().unwrap();
        let path = case_path(&id, &expected, &tmp);
        if let Ok(loaded) = load(&path) {
            found.push((id, tmp, loaded.skill));
        }
    }
    assert!(found.len() > 30, "found {}", found.len());
    found
}

#[test]
fn render_field_order_is_spec_order_then_extensions() {
    let (_tmp, loaded) = load_text(
        "my-skill",
        "---\nzeta: 1\nallowed-tools: Bash\nmetadata:\n  k: v\ncompatibility: Python\nlicense: MIT\ndescription: d\nname: my-skill\nalpha: x\n---\nBody\n",
    );
    assert_eq!(
        loaded.render(),
        "---\nname: my-skill\ndescription: d\nlicense: MIT\ncompatibility: Python\nmetadata:\n  k: v\nallowed-tools: Bash\nzeta: 1\nalpha: x\n---\nBody\n"
    );
}

#[test]
fn render_omits_absent_optional_fields_and_empty_metadata() {
    assert_eq!(
        skill("d").render(),
        "---\nname: my-skill\ndescription: d\n---\nBody\n"
    );
}

#[test]
fn render_metadata_keys_sorted() {
    let mut s = skill("d");
    for key in ["b", "a", "B", "é", "a-b"] {
        s.properties
            .metadata
            .insert(key.to_string(), "v".to_string());
    }
    assert_eq!(
        s.render(),
        "---\nname: my-skill\ndescription: d\nmetadata:\n  B: v\n  a: v\n  a-b: v\n  b: v\n  é: v\n---\nBody\n"
    );
}

#[test]
fn render_quoting_table() {
    let cases: &[(&str, &str)] = &[
        // 1: control characters, double-quoted
        ("a\tb", "\"a\\tb\""),
        ("a\nb", "\"a\\nb\""),
        ("a\rb", "\"a\\rb\""),
        ("a\u{1}b", "\"a\\x01b\""),
        ("a\u{7f}b", "\"a\\x7fb\""),
        ("a\u{85}b", "\"a\\x85b\""),
        ("a\u{feff}b", "\"a\\ufeffb\""),
        ("a\u{2028}b", "\"a\\u2028b\""),
        ("a\u{2029}b", "\"a\\u2029b\""),
        ("q\"\\\n", "\"q\\\"\\\\\\n\""),
        // 2: empty
        ("", "''"),
        // 3: surrounding whitespace
        (" a", "' a'"),
        ("a ", "'a '"),
        // 4: first character
        ("- a", "'- a'"),
        ("? a", "'? a'"),
        (":a", "':a'"),
        (",a", "',a'"),
        ("[a]", "'[a]'"),
        ("]a", "']a'"),
        ("{a}", "'{a}'"),
        ("}a", "'}a'"),
        ("#a", "'#a'"),
        ("&a", "'&a'"),
        ("*a", "'*a'"),
        ("!a", "'!a'"),
        ("|a", "'|a'"),
        (">a", "'>a'"),
        ("'a", "'''a'"),
        ("\"a", "'\"a'"),
        ("%a", "'%a'"),
        ("@a", "'@a'"),
        ("`a", "'`a'"),
        ("...x", "'...x'"),
        // 5: inner indicators
        ("a: b", "'a: b'"),
        ("a #b", "'a #b'"),
        ("a:", "'a:'"),
        ("it's: x", "'it''s: x'"),
        // 6: non-string scalars of YAML 1.1 and 1.2
        ("true", "'true'"),
        ("-0x1f", "'-0x1f'"),
        ("+0o17", "'+0o17'"),
        ("0o1_7", "'0o1_7'"),
        ("False", "'False'"),
        ("NULL", "'NULL'"),
        ("~", "'~'"),
        ("yes", "'yes'"),
        ("On", "'On'"),
        ("n", "'n'"),
        ("42", "'42'"),
        ("-1", "'-1'"),
        ("+7", "'+7'"),
        ("1.0", "'1.0'"),
        ("1.", "'1.'"),
        (".5", "'.5'"),
        ("1e3", "'1e3'"),
        ("-2.5E-3", "'-2.5E-3'"),
        ("0x1F", "'0x1F'"),
        ("0o17", "'0o17'"),
        (".inf", "'.inf'"),
        ("-.INF", "'-.INF'"),
        (".NaN", "'.NaN'"),
        ("2024-01-01", "'2024-01-01'"),
        ("2024-1-1 10:00:00.5 +1", "'2024-1-1 10:00:00.5 +1'"),
        ("1:30", "'1:30'"),
        ("-190:20:30.15", "'-190:20:30.15'"),
        ("1_000", "'1_000'"),
        ("1_0.5", "'1_0.5'"),
        ("0b101", "'0b101'"),
        ("0x1_F", "'0x1_F'"),
        // 7: plain
        ("1:75", "1:75"),
        ("2024-01", "2024-01"),
        ("12:30pm", "12:30pm"),
        ("hello world", "hello world"),
        ("café", "café"),
        ("技能 skill", "技能 skill"),
        ("a'b", "a'b"),
        ("say \"hi\"", "say \"hi\""),
        ("a-b", "a-b"),
        ("a#b", "a#b"),
        ("http://x.y/z", "http://x.y/z"),
        ("v1", "v1"),
        ("1.2.3", "1.2.3"),
        ("nope", "nope"),
        ("0x", "0x"),
        ("e5", "e5"),
        ("1e", "1e"),
        ("-", "'-'"),
    ];
    for (value, expected) in cases {
        assert_eq!(&scalar_line(value), expected, "value {value:?}");
    }
}

#[test]
fn render_quotes_extension_keys_by_the_same_table() {
    let (_tmp, loaded) = load_text(
        "my-skill",
        "---\nname: my-skill\ndescription: d\n'#k': 1\n\"x y\": 2\n'yes': 3\nplain-key: 4\n---\n",
    );
    assert_eq!(
        loaded.render(),
        "---\nname: my-skill\ndescription: d\n'#k': 1\nx y: 2\n'yes': 3\nplain-key: 4\n---\n"
    );
}

#[test]
fn render_preserves_plain_extension_scalars() {
    let rendered: Vec<String> = ["house-unknown-key", "house-empty-extension"]
        .iter()
        .map(|id| {
            let tmp = TempDir::new().unwrap();
            let path = case_path(id, &read_expected(id), &tmp);
            let source = fs::read_to_string(path.join("SKILL.md")).unwrap();
            let rendered = load(&path).unwrap().skill.render();
            let extension = source
                .lines()
                .find(|l| l.starts_with("disable-model") || l.starts_with("foo:"))
                .unwrap();
            assert!(
                rendered.lines().any(|l| l == extension),
                "{id}: {rendered:?}"
            );
            rendered
        })
        .collect();
    assert!(rendered[0].contains("\ndisable-model-invocation: true\n"));
    assert!(rendered[1].contains("\nfoo:\n"));

    let (_tmp, loaded) = load_text(
        "my-skill",
        "---\nname: my-skill\ndescription: d\nfoo: 'true'\nbar: true\n---\n",
    );
    assert!(loaded.render().contains("\nfoo: 'true'\nbar: true\n"));
}

#[test]
fn render_nested_extension_collections() {
    let source = "---\nname: my-skill\ndescription: d\next:\n  a: 1\n  list:\n    - x\n    -\n      nested: y\n      deeper:\n        - z\n---\n";
    let (_tmp, loaded) = load_text("my-skill", source);
    assert_eq!(loaded.render(), source);
}

#[test]
fn render_body_verbatim() {
    for body in [
        "",
        "no newline",
        "\n\n  leading blanks\n",
        "crlf\r\nlines\r\n",
        "---\nnot frontmatter\n",
        "\t tab \n\n\n",
    ] {
        let mut s = skill("d");
        s.body = body.to_string();
        let rendered = s.render();
        assert!(rendered.ends_with(&format!("---\n{body}")), "{body:?}");
        let reloaded = load_text("my-skill", &rendered).1;
        assert_eq!(reloaded.body, body);
    }
}

#[test]
fn render_is_deterministic() {
    let s = skill("same");
    assert_eq!(s.render(), s.render());
    assert_eq!(s.clone().render(), s.render());
}

#[test]
fn load_render_round_trip_on_every_valid_fixture() {
    for (id, _tmp, original) in loadable_fixtures() {
        let (_dir, reloaded) = load_text(original.properties.name.as_str(), &original.render());
        assert_eq!(reloaded, original, "{id}");
    }
}

#[test]
fn render_is_idempotent() {
    for (id, _tmp, original) in loadable_fixtures() {
        let once = original.render();
        let (_dir, reloaded) = load_text(original.properties.name.as_str(), &once);
        assert_eq!(reloaded.render(), once, "{id}");
    }
}

#[test]
fn extensions_byte_stable_after_description_edit() {
    let id = "house-unknown-key";
    let tmp = TempDir::new().unwrap();
    let mut loaded = load(&case_path(id, &read_expected(id), &tmp))
        .unwrap()
        .skill;
    let before = loaded.render();
    loaded.properties.description = "A changed description".to_string();
    let after = loaded.render();
    let changed: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(b, a)| b != a)
        .collect();
    assert_eq!(before.lines().count(), after.lines().count());
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].1.starts_with("description:"));
    assert!(after.contains("\ndisable-model-invocation: true\n"));
}

#[test]
fn write_to_creates_name_dir_and_uppercase_file() {
    let tmp = TempDir::new().unwrap();
    let s = skill("d");
    let path = s.write_to(tmp.path()).unwrap();
    assert_eq!(path, tmp.path().join("my-skill").join("SKILL.md"));
    assert_eq!(fs::read_to_string(&path).unwrap(), s.render());
    let names: Vec<String> = fs::read_dir(tmp.path().join("my-skill"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["SKILL.md"]);
    assert!(validate(&tmp.path().join("my-skill")).is_empty());

    // Writing again overwrites; a missing parent is created.
    let nested = tmp.path().join("a").join("b");
    assert!(s.write_to(&nested).unwrap().is_file());
}

#[test]
fn render_matches_golden() {
    let mut checked = 0;
    for id in fixture_ids() {
        let golden = fixtures_dir().join(&id).join("expected.render.md");
        if !golden.is_file() {
            continue;
        }
        let expected = read_expected(&id);
        let tmp = TempDir::new().unwrap();
        let loaded =
            load(&case_path(&id, &expected, &tmp)).unwrap_or_else(|_| panic!("{id}: skipped"));
        let want = fs::read(&golden).unwrap();
        assert_eq!(loaded.skill.render().into_bytes(), want, "{id}");
        checked += 1;
    }
    assert!(checked >= 24, "checked {checked}");
}

#[test]
fn render_round_trips_keys_longer_than_a_simple_key() {
    for length in [1024, 1025, 1500] {
        let key = "k".repeat(length);
        let mut s = skill("d");
        s.properties.metadata.insert(key.clone(), "v".to_string());
        let (_tmp, loaded) = load_text("my-skill", &s.render());
        assert_eq!(
            loaded.properties.metadata.get(&key).map(String::as_str),
            Some("v"),
            "{length}"
        );
        assert_eq!(loaded.render(), s.render(), "{length}");
    }
}
