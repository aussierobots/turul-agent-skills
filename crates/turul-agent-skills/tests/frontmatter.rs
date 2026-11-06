//! `parse_skill_md`: delimiters, the YAML subset, the colon fallback.

use turul_agent_skills::{ParseError, RawSkill, RawValue, parse_skill_md};

fn parse(content: &str) -> RawSkill {
    parse_skill_md(content).unwrap_or_else(|e| panic!("expected ok, got {e:?} for {content:?}"))
}

fn code(content: &str) -> &'static str {
    parse_skill_md(content)
        .expect_err("expected an error")
        .code()
        .as_str()
}

fn text<'a>(raw: &'a RawSkill, key: &str) -> &'a str {
    match raw.entries.iter().find(|(k, _)| k == key) {
        Some((_, RawValue::Str { text, .. })) => text,
        other => panic!("{key}: not a string entry: {other:?}"),
    }
}

fn keys(raw: &RawSkill) -> Vec<&str> {
    raw.entries.iter().map(|(k, _)| k.as_str()).collect()
}

#[test]
fn delimiter_first_line_must_be_exactly_dashes() {
    assert_eq!(code("---x\nname: a\n---\n"), "frontmatter-missing");
    assert_eq!(code(" ---\nname: a\n---\n"), "frontmatter-missing");
    assert_eq!(code("\n---\nname: a\n---\n"), "frontmatter-missing");
    assert_eq!(code(""), "frontmatter-missing");
    assert_eq!(code("# No frontmatter here\n"), "frontmatter-missing");
}

#[test]
fn delimiter_closing_is_first_exact_dash_line() {
    let raw = parse("---\nname: a\n---\nbody\n---\nmore\n");
    assert_eq!(keys(&raw), ["name"]);
    assert_eq!(raw.body, "body\n---\nmore\n");
    // `----` and `--- x` are not delimiters.
    assert_eq!(code("---\nname: a\n----\n--- x\n"), "frontmatter-unclosed");
}

#[test]
fn unclosed_frontmatter_is_error() {
    assert_eq!(code("---\nname: a\n"), "frontmatter-unclosed");
    assert_eq!(code("---"), "frontmatter-unclosed");
}

#[test]
fn dashes_inside_quoted_value_do_not_close() {
    let raw = parse("---\nname: a\ndescription: \"x --- y\"\n---\nbody");
    assert_eq!(text(&raw, "description"), "x --- y");
    assert_eq!(raw.body, "body");
}

#[test]
fn closing_at_eof_without_newline() {
    assert_eq!(parse("---\nname: a\n---").body, "");
    assert_eq!(parse("---\nname: a\n---\n").body, "");
    assert_eq!(parse("---\r\nname: a\r\n---\r").body, "");
}

#[test]
fn crlf_line_endings_accepted() {
    let raw = parse("---\r\nname: a\r\ndescription: b\r\n---\r\nbody\r\n");
    assert_eq!(text(&raw, "name"), "a");
    assert_eq!(text(&raw, "description"), "b");
    assert_eq!(raw.body, "body\r\n");
}

#[test]
fn body_preserved_verbatim() {
    let raw = parse("---\nname: a\n---\n\n\n  # Title  \n\ttext\n\n");
    assert_eq!(raw.body, "\n\n  # Title  \n\ttext\n\n");
}

#[test]
fn yaml_scalars_are_strings() {
    let raw = parse("---\na: 1\nb: true\nc: null\nd: 1.0\ne: 1.00\nf: 'x'\n---\n");
    assert_eq!(text(&raw, "a"), "1");
    assert_eq!(text(&raw, "b"), "true");
    assert_eq!(text(&raw, "c"), "null");
    assert_eq!(text(&raw, "d"), "1.0");
    assert_eq!(text(&raw, "e"), "1.00");
    assert_eq!(text(&raw, "f"), "x");
}

#[test]
fn empty_value_is_empty_string() {
    assert_eq!(text(&parse("---\nname:\n---\n"), "name"), "");
    assert_eq!(text(&parse("---\nname: ''\n---\n"), "name"), "");
}

#[test]
fn flow_collections_rejected() {
    assert_eq!(code("---\nname: [invalid\n---\n"), "yaml-invalid");
    assert_eq!(code("---\nm: {a: b}\n---\n"), "yaml-invalid");
    assert_eq!(code("---\nm: [a, b]\n---\n"), "yaml-invalid");
    assert_eq!(code("---\nm:\n  k: {a: b}\n---\n"), "yaml-invalid");
}

#[test]
fn anchors_aliases_tags_rejected() {
    assert_eq!(code("---\na: &x foo\n---\n"), "yaml-invalid");
    assert_eq!(code("---\na: &x foo\nb: *x\n---\n"), "yaml-invalid");
    assert_eq!(code("---\na: !!str foo\n---\n"), "yaml-invalid");
    assert_eq!(code("---\na: !custom foo\n---\n"), "yaml-invalid");
}

#[test]
fn duplicate_keys_rejected() {
    assert_eq!(code("---\na: x\na: y\n---\n"), "yaml-invalid");
    assert_eq!(code("---\nm:\n  k: x\n  k: y\n---\n"), "yaml-invalid");
}

#[test]
fn non_scalar_key_rejected() {
    assert_eq!(code("---\n? [a]\n: b\n---\n"), "yaml-invalid");
    assert_eq!(code("---\n? - a\n: b\n---\n"), "yaml-invalid");
    assert_eq!(code("---\n? k: v\n: b\n---\n"), "yaml-invalid");
}

#[test]
fn non_mapping_frontmatter_rejected() {
    assert_eq!(code("---\n---\nbody"), "frontmatter-not-mapping");
    assert_eq!(
        code("---\n# only a comment\n---\n"),
        "frontmatter-not-mapping"
    );
    assert_eq!(code("---\njust a scalar\n---\n"), "frontmatter-not-mapping");
    assert_eq!(code("---\n- a\n- b\n---\n"), "frontmatter-not-mapping");
}

#[test]
fn block_scalars_follow_yaml() {
    let raw = parse("---\nliteral: |\n  one\n  two\nfolded: >\n  one\n  two\n---\n");
    assert_eq!(text(&raw, "literal"), "one\ntwo\n");
    assert_eq!(text(&raw, "folded"), "one two\n");
}

#[test]
fn nested_collections_are_kept() {
    let raw = parse("---\nmetadata:\n  author: me\n  version: 1.0\nlist:\n  - a\n  - b\n---\n");
    let str_value = |t: &str| RawValue::Str {
        text: t.to_string(),
        typed: false,
    };
    assert_eq!(
        raw.entries[0].1,
        RawValue::Map(vec![
            ("author".to_string(), str_value("me")),
            (
                "version".to_string(),
                RawValue::Str {
                    text: "1.0".to_string(),
                    typed: true
                }
            ),
        ])
    );
    assert_eq!(
        raw.entries[1].1,
        RawValue::Seq(vec![str_value("a"), str_value("b")])
    );
}

#[test]
fn unknown_keys_retained_in_source_order() {
    let raw = parse("---\nzeta: 1\nname: a\ndisable-model-invocation: true\ndescription: d\n---\n");
    assert_eq!(
        keys(&raw),
        ["zeta", "name", "disable-model-invocation", "description"]
    );
}

#[test]
fn allowed_tools_kept_as_string() {
    let raw = parse("---\nallowed-tools: Bash(jq:*) Bash(git:*)\n---\n");
    assert_eq!(text(&raw, "allowed-tools"), "Bash(jq:*) Bash(git:*)");
}

#[test]
fn typed_flag_marks_plain_non_string_scalars() {
    let raw = parse(
        "---\na: true\nb:\nc: 42\nd: 1.5\ne: ~\nf: 'true'\ng: \"1\"\nh: hello\ni: 0x1F\nj: .inf\nk: yes\n---\n",
    );
    let typed = |key: &str| match &raw.entries.iter().find(|(k, _)| k == key).unwrap().1 {
        RawValue::Str { typed, .. } => *typed,
        other => panic!("{other:?}"),
    };
    for key in ["a", "b", "c", "d", "e", "i", "j", "k"] {
        assert!(typed(key), "{key} should be typed");
    }
    for key in ["f", "g", "h"] {
        assert!(!typed(key), "{key} should not be typed");
    }
}

#[test]
fn colon_fallback_repairs_unquoted_colon() {
    let raw = parse(
        "---\nname: a\ndescription: Use this skill when: the user asks about PDFs\n---\nbody",
    );
    assert_eq!(
        text(&raw, "description"),
        "Use this skill when: the user asks about PDFs"
    );
    assert!(raw.repaired.is_some());
    assert_eq!(raw.body, "body");

    let ends_with_colon = parse("---\nname: a\ndescription: Steps follow:\nx: it's: fine\n---\n");
    assert_eq!(text(&ends_with_colon, "description"), "Steps follow:");
    assert_eq!(text(&ends_with_colon, "x"), "it's: fine");
}

#[test]
fn colon_fallback_never_applies_to_valid_yaml() {
    let raw = parse("---\nname: a\ndescription: \"Use when: ok\"\nurl: http://x.y/z\n---\n");
    assert_eq!(raw.repaired, None);
    assert_eq!(text(&raw, "description"), "Use when: ok");
}

#[test]
fn colon_fallback_top_level_only() {
    let error = parse_skill_md("---\nname: a\nmetadata:\n  note: Use when: x\n---\n")
        .expect_err("indented lines are not repaired");
    assert!(matches!(error, ParseError::InvalidYaml(_)), "{error:?}");
}

#[test]
fn parse_never_panics() {
    let pieces = [
        "---", "\n", "\r\n", "name", ":", " ", "[", "{", "}", "]", "&a", "*a", "!t", "'", "\"",
        "- ", "? ", "|", ">", "#", "é", "\t", "\u{0}", "x", "...", "%",
    ];
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    for _ in 0..3000 {
        let mut input = String::from("---\n");
        for _ in 0..(state % 12) {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            input.push_str(pieces[(state >> 33) as usize % pieces.len()]);
        }
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        if state >> 62 != 0 {
            input.push_str("\n---\n");
        }
        let _ = parse_skill_md(&input);
    }
}

#[test]
fn input_after_the_first_document_is_rejected() {
    assert_eq!(
        code("---\nname: a\ndescription: b\n...\nevil: [\n---\n"),
        "yaml-invalid"
    );
    assert_eq!(
        code("---\nname: a\ndescription: b\n...\n!!! garbage [\n---\nbody"),
        "yaml-invalid"
    );
    assert_eq!(
        code("---\nname: a\ndescription: b\n--- \nextra: 1\n---\n"),
        "yaml-invalid"
    );
    assert_eq!(
        code("---\nname: a\ndescription: b\n...\nfoo: bar\n---\n"),
        "yaml-invalid"
    );
}

#[test]
fn colon_fallback_leaves_commented_lines_alone() {
    let raw = parse("---\nname: a\ndescription: Use when: x\nlicense: MIT # see: LICENSE\n---\n");
    assert!(raw.repaired.is_some());
    assert_eq!(text(&raw, "description"), "Use when: x");
    assert_eq!(text(&raw, "license"), "MIT");
    let raw = parse("---\nname: a\ndescription: Use when: x # why\n---\n");
    assert_eq!(text(&raw, "description"), "Use when: x");
}

#[test]
fn deeply_repeated_collections_parse_in_linear_time() {
    let mut yaml = String::from("---\nx:\n");
    for _ in 0..40_000 {
        yaml.push_str("  - - a\n");
    }
    yaml.push_str("---\n");
    let start = std::time::Instant::now();
    let _ = parse_skill_md(&yaml);
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}

fn nested(depth: usize) -> String {
    let mut yaml = String::from("---\nname: a\ndescription: d\nx:\n");
    for level in 1..depth {
        yaml.push_str(&"  ".repeat(level));
        yaml.push_str("k:\n");
    }
    yaml.push_str(&"  ".repeat(depth));
    yaml.push_str("k: v\n---\n");
    yaml
}

#[test]
fn nesting_deeper_than_64_levels_rejected() {
    assert!(parse_skill_md(&nested(63)).is_ok());
    assert_eq!(code(&nested(70)), "yaml-invalid");
}
