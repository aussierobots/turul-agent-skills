//! Scalars resolve under the YAML 1.2 core schema, and report whether YAML 1.1
//! reads the same value.

use turul_agent_skills::{CoreScalar, RawValue, parse_skill_md};

/// The value of `k` in a one-entry frontmatter.
fn value(yaml_value: &str) -> RawValue {
    let raw = parse_skill_md(&format!("---\nk: {yaml_value}\n---\n")).unwrap();
    raw.entries.into_iter().next().unwrap().1
}

fn resolve(yaml_value: &str) -> (CoreScalar, bool) {
    let resolved = value(yaml_value)
        .core_scalar()
        .unwrap_or_else(|| panic!("{yaml_value:?} is not a scalar"));
    (resolved.value, resolved.yaml_1_1_agrees)
}

#[test]
fn core_schema_values_and_yaml_1_1_agreement() {
    use CoreScalar::*;
    let cases: &[(&str, CoreScalar, bool)] = &[
        // null
        ("", Null, true),
        ("~", Null, true),
        ("null", Null, true),
        ("NULL", Null, true),
        // bool
        ("true", Bool(true), true),
        ("False", Bool(false), true),
        ("yes", Str, false),
        ("No", Str, false),
        ("on", Str, false),
        ("OFF", Str, false),
        ("y", Str, false),
        ("n", Str, false),
        // int
        ("0", Int(0), true),
        ("42", Int(42), true),
        ("-17", Int(-17), true),
        ("+5", Int(5), true),
        ("0x1F", Int(31), true),
        ("0o17", Int(15), false),
        ("0777", Int(777), false),
        ("08", Int(8), false),
        ("9223372036854775807", Int(i64::MAX), true),
        ("9223372036854775808", IntOutOfRange, true),
        (
            "170141183460469231731687303715884105728",
            IntOutOfRange,
            false,
        ),
        // float
        ("1.0", Float(1.0), true),
        ("-2.5", Float(-2.5), true),
        (".5", Float(0.5), true),
        ("-.5", Float(-0.5), false),
        ("1.", Float(1.0), true),
        ("1.0e+3", Float(1000.0), true),
        ("1e3", Float(1000.0), false),
        ("1.0e3", Float(1000.0), false),
        (".inf", Float(f64::INFINITY), true),
        ("-.Inf", Float(f64::NEG_INFINITY), true),
        // strings in YAML 1.2 that YAML 1.1 types
        ("1_000", Str, false),
        ("1:30", Str, false),
        ("1:30.5", Str, false),
        ("2024-01-01", Str, false),
        ("0b101", Str, false),
        ("-0x1F", Str, false),
        // strings in both
        ("hello", Str, true),
        ("1.2.3", Str, true),
        ("v1", Str, true),
        ("+0o17", Str, true),
        ("'1.0'", Str, true),
        ("\"yes\"", Str, true),
        ("'0777'", Str, true),
    ];
    for &(text, expected, agrees) in cases {
        assert_eq!(resolve(text), (expected, agrees), "value {text:?}");
    }
}

#[test]
fn nan_resolves_and_agrees() {
    let (value, agrees) = resolve(".nan");
    assert!(matches!(value, CoreScalar::Float(f) if f.is_nan()));
    assert!(agrees);
}

#[test]
fn collections_are_not_scalars() {
    assert_eq!(value("\n  - a\n  - b").core_scalar(), None);
    assert_eq!(value("\n  a: b").core_scalar(), None);
}

#[test]
fn every_value_that_is_not_a_plain_string_in_both_schemas_is_typed() {
    // The `typed` flag (render quotes these) covers every scalar the resolver
    // does not call a string that both schemas agree on.
    for text in [
        "",
        "~",
        "true",
        "yes",
        "y",
        "42",
        "0777",
        "1.0",
        "1e3",
        "1:30",
        "2024-01-01",
        "1_000",
        ".inf",
        ".nan",
        "0x1F",
        "0o17",
        "-0x1F",
        "+0o17",
    ] {
        let RawValue::Str { typed, .. } = value(text) else {
            panic!("{text:?}")
        };
        assert!(typed, "{text:?} should be typed");
    }
    for text in ["hello", "1.2.3", "v1", "'1.0'"] {
        let resolved = value(text).core_scalar().unwrap();
        assert_eq!(
            (resolved.value, resolved.yaml_1_1_agrees),
            (CoreScalar::Str, true),
            "{text:?}"
        );
    }
}
