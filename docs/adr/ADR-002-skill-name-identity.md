# ADR-002: SkillName identity

- **Status:** Accepted (2026-11-06; contract tests `skill_name.rs`, `single_owner.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/skill_name.rs`, `crates/turul-agent-skills/tests/single_owner.rs` (workspace only, not published)
- **Depends on:** ADR-001 (sources, D3 resolution)

## 1. Context

A skill's `name` is its identity: the catalog key, the activation key, the collision key in
discovery and the directory `write_to` creates. If any caller can build a name from a raw
`String`, two spellings of one skill become two skills.

The spec says names "may only contain unicode lowercase alphanumeric characters (`a-z`, `0-9`)
and hyphens", must not start or end with a hyphen, must not contain `--`, are 1–64 characters,
and must match the parent directory (`docs/specification.mdx` "`name` field"). `skills-ref`
`validator.py::_validate_name` strips, applies NFKC, then checks lowercase via
`name == name.lower()` and characters via `c.isalnum() or c == "-"`. D3: follow `skills-ref`.

The client guide relaxes length and directory match for lenient loading ("Name doesn't match
the parent directory name → warn, load anyway"; "Name exceeds 64 characters → warn, load
anyway"). Its note also lists the character set among the relaxed constraints; this crate
does not relax that one (ADR-004 §3, ADR-001 §2.3). A loaded skill must still hold a typed
name, so length and directory match cannot live in the identity type.

## 2. Decision

### 2.1 Owner

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Skill identity | `SkillName` (private `String` field) | `SkillName::parse(&str) -> Result<SkillName, NameError>` | `src/name.rs` |
| Name rule failures | `NameError { normalized: String, violations: Vec<NameViolation> }` (`#[non_exhaustive]`, as is `NameViolation`); implements `Error`, `Display` = every violation message joined by `; ` | returned by `SkillName::parse` | `src/name.rs` |
| Name rule code and message text | `NameViolation::code()`, `NameViolation::message(&normalized)` (crate-private; `validate` consumes them) | used by `NameError`'s `Display` and the field check | `src/name.rs` |

`SkillName` implements `Clone, Debug, Display, PartialEq, Eq, Hash, PartialOrd, Ord` and
`as_str() -> &str`. It implements no `From<String>`, `From<&str>`, `FromStr`, `Default` or
`Deserialize`. `Display` writes the normalized form. `Ord` is byte order of the normalized form.

`NameError.normalized` is the trimmed NFKC form; validation (ADR-004) uses it for the length
check, the directory check and the message text, so the normalization exists once.

### 2.2 Canonical form

`parse(s)`: trim (`str::trim`, Unicode `White_Space`) → NFKC (`unicode-normalization`) →
check. The stored value is the result of NFKC. Two inputs that are equal after trim + NFKC are
the same `SkillName`.

### 2.3 Identity rules

Checked in this order; every failing rule is reported (strict validation reports all of them,
ADR-004). `Empty` short-circuits.

| # | Violation | Rule (on the normalized form) | Reference | Test |
|---|---|---|---|---|
| 1 | `Empty` | normalized form is empty | `_validate_name` first branch | `parse_rejects_empty_and_whitespace` |
| 2 | `NotLowercase` | `s != s.to_lowercase()` | `name != name.lower()` | `parse_rejects_uppercase` |
| 3 | `HyphenEdge` | starts or ends with `-` | `startswith("-") or endswith("-")` | `parse_rejects_leading_or_trailing_hyphen` |
| 4 | `ConsecutiveHyphens` | contains `--` | `"--" in name` | `parse_rejects_consecutive_hyphens` |
| 5 | `InvalidChars` | some char is neither `-` nor of general category `Lu Ll Lt Lm Lo Nd Nl No` | `c.isalnum() or c == "-"` | `parse_rejects_invalid_chars` |

Rule 5 uses the general category from `unicode-properties` (feature `general-category` only).
Evidence: over all code points 0..=U+10FFFF, Python 3.14.7 (Unicode 16.0.0)
`str.isalnum()` equals "category in {Lu, Ll, Lt, Lm, Lo, Nd, Nl, No}" with zero exceptions.
`char::is_alphanumeric` is not used: it uses the `Alphabetic` property, which adds
`Other_Alphabetic` marks (for example Devanagari U+093F), which Python rejects.

Positive and identity tests:

| Rule | Test |
|---|---|
| Unicode lowercase letters accepted (`技能`, `мой-навык`, `навык`) | `parse_accepts_unicode_lowercase` |
| Trim then NFKC: `cafe` + U+0301 == `café` | `parse_trims_then_nfkc_normalizes` |
| Length is not an identity rule: `a`×70 parses | `parse_accepts_overlong` |
| Distinct normalized inputs give distinct names; NFKC-equal inputs give equal names and hashes | `distinct_inputs_distinct_names` |
| `parse(name.to_string()) == name`; `Display == as_str()` | `display_round_trips` |
| `Ord` is byte order of the normalized form | `ord_is_byte_order` |
| No constructor bypass: `SkillName(String::new())` and `SkillName::from(String::new())` do not compile | `compile_fail` doctests on `SkillName` |

### 2.4 Not identity rules

| Rule | Owner | Why |
|---|---|---|
| Length ≤ 64 characters | ADR-004 `name-too-long` | Lenient load warns and keeps the skill |
| Equals NFKC of the parent directory name | ADR-004 `name-dir-mismatch` | Lenient load warns and keeps the skill; a skill built in memory has no directory |

### 2.5 Owner routing

`single_owner.rs` scans `src/` of both crates and `examples/` (not `tests/`). Each rule
is one test:

| Rule | Test |
|---|---|
| `SkillName(` appears only in `src/name.rs` (comment lines ignored) | `skill_name_constructed_only_in_name_rs` |
| `SkillProperties {` literal appears only in `src/parse.rs` | `skill_properties_literal_only_in_parse_rs` |
| `SkillProperties` declares exactly the spec fields `name, description, license, compatibility, metadata, allowed_tools` | `skill_properties_fields_are_spec_fields` |
| `parse_skill_md(` is called only from `src/validate.rs` within the crates' `src/` (internal routing; `parse_skill_md` stays public, and examples may call it) | `parse_skill_md_called_only_from_validate` |
| `src/discover.rs` obtains skills only through `load(` | `discover_calls_load_only` |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| ASCII `[a-z0-9-]` (spec's parenthetical) | D3: 5 corpus cases need Unicode; the spec text says "unicode" |
| `char::is_alphanumeric` | Accepts combining marks Python rejects; would diverge on Indic names |
| Length and directory match inside `parse` | Lenient load could not hold a typed name for a skill the client guide says to load |
| Store the input as written, compare NFKC on demand | Two spellings of one name would be two keys in maps and in discovery |
| `FromStr` / `TryFrom<String>` | Second entrypoint for the same rules |

## 4. Consequences

- New workspace dependencies: `unicode-normalization = "0.1"` (0.1.26 evaluated: Unicode 18.0.0;
  dep `tinyvec`) and `unicode-properties = "0.1"` with `default-features = false`,
  `features = ["general-category"]` (0.1.5 evaluated: Unicode 18.0.0; no deps). Python's tables are Unicode 16.0.0, so
  only code points assigned after 16.0 can differ.
- Every other module takes a `SkillName`, never a `&str` name.
- `read-properties` and `to-prompt` print the NFKC form; `skills-ref` prints the stripped,
  unnormalized form. Recorded in ADR-001 §2.3.

## 5. Revision log

- 2026-11-06 — Initial. Unicode general-category rule with evidence; length and directory match moved to validation.
- 2026-11-06 — Dependencies stated as major.minor workspace requirements; evaluated versions kept as evidence.
- 2026-11-06 — Context notes the client guide also relaxes the character set; not followed (divergence in ADR-001).
- 2026-11-06 — Accepted: `skill_name.rs`, `single_owner.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only).
- 2026-11-06 — `NameError` implements `Error` and `Display` (`thiserror`); rule codes and message text have one owner in `name.rs`, `validate.rs` consumes them. Test `name_error_displays_every_violation_message`.
- 2026-11-06 — `parse_skill_md_called_only_from_validate_and_load` is scoped to the crates' `src/`; the allowance for the `smoke` example is deleted (the rule is internal routing; `parse_skill_md` stays public). The `SkillName(` check ignores comment lines like the `SkillProperties` check.
- 2026-11-06 — `NameViolation` and `NameError` are `#[non_exhaustive]`; `BuildError::Name` wraps `NameError` (ADR-004).
- 2026-11-07 — `parse_skill_md(` routing narrowed to `src/validate.rs` (`load.rs` reaches it through `read_and_parse` and `parse_text`); test renamed `parse_skill_md_called_only_from_validate`.
- 2026-11-07 — `single_owner.rs` reads every crate and the examples, so it is excluded from the published package and runs under `cargo test --workspace` only (ADR-001).
