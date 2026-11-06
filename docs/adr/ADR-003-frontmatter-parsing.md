# ADR-003: Frontmatter parsing and YAML backend

- **Status:** Accepted (2026-11-06; contract tests `frontmatter.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/frontmatter.rs`
- **Depends on:** ADR-001 (sources), ADR-002 (name is not parsed here, only carried as text); D4, D7

## 1. Context

`skills-ref` `parser.py::parse_frontmatter` requires `content.startswith("---")`, splits with
`content.split("---", 2)`, parses the middle part with `strictyaml.load(...)` (no schema) and
returns `parts[2].strip()` as the body. `metadata` values are coerced with `str(v)`.

strictyaml with no schema was probed at 1.7.3 (the version in `skills-ref/uv.lock`), in an
isolated `uv` environment running only a probe script written for this ADR:

| Input | strictyaml 1.7.3 result |
|---|---|
| `a: 1`, `b: true`, `c: null`, `d: 1.0` | all strings: `"1"`, `"true"`, `"null"`, `"1.0"` |
| `name:` | `""` |
| `name: [invalid` / `m: {a: b}` | `FlowMappingDisallowed` |
| `a: x` + `a: y` | `DuplicateKeysDisallowed` |
| `a: &x foo` / `a: !!str foo` | `AnchorTokenDisallowed` / `TagTokenDisallowed` |
| `description: Use when: the user asks` | `ScannerError` (mapping values are not allowed here) |
| `description: "a --- b"` | `"a --- b"` |
| empty, `# comment`, plain scalar, list | not a mapping |
| nested map / block list | kept as map / list of strings |

The client guide ("Frontmatter extraction") says: opening `---` at the start, closing `---`
after it, body = everything after the closing delimiter, trimmed. "Handling malformed YAML"
suggests a fallback that quotes values containing colons and retries.

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Parsed document | `RawSkill { entries: Vec<(String, RawValue)>, body: String, repaired: Option<String> }` | `parse_skill_md(&str) -> Result<RawSkill, ParseError>` | `src/parse.rs` |
| YAML value model | `RawValue::{Str { text: String, typed: bool }, Seq(Vec<RawValue>), Map(Vec<(String, RawValue)>)}` | built by `parse_skill_md`; reaches a `Skill` only through `load` (ADR-004) | `src/parse.rs` |
| Spec fields | `SkillProperties { name: SkillName, description, license, compatibility, metadata: BTreeMap<String, String>, allowed_tools }`, `#[non_exhaustive]`, plus `SkillProperties::new(SkillName, impl Into<String>)` | literal only in `src/parse.rs`; filled by the field check in ADR-004 | `src/parse.rs` |
| Parse failures | `ParseError::{FrontmatterMissing, FrontmatterUnclosed, InvalidYaml(String), NotMapping}` with `code()` from the shared `ErrorCode` | returned by `parse_skill_md` | `src/parse.rs`, codes in `src/error.rs` |

`parse_skill_md` is purely syntactic. Required fields, unknown keys, lengths and types are
ADR-004. `entries` keeps every top-level key in source order, spec or not (D4).
`repaired` is `Some(original YAML error)` when the colon fallback (§2.4) was needed.

### 2.2 Delimiters and body

| # | Rule | Error code | Test |
|---|---|---|---|
| 1 | Line 1 is exactly `---` (a trailing `\r` is removed first) | `frontmatter-missing` | `delimiter_first_line_must_be_exactly_dashes` |
| 2 | The closing delimiter is the first later line that is exactly `---` (after removing `\r`) | `frontmatter-unclosed` if none | `delimiter_closing_is_first_exact_dash_line`, `unclosed_frontmatter_is_error` |
| 3 | `---` inside a value never closes the block | — | `dashes_inside_quoted_value_do_not_close` |
| 4 | Closing `---` may be the last line, with or without a newline | — | `closing_at_eof_without_newline` |
| 5 | LF and CRLF are both accepted | — | `crlf_line_endings_accepted` |
| 6 | Body = the bytes after the closing line's terminator, verbatim (not trimmed) | — | `body_preserved_verbatim` |

Rules 1–3 diverge from `skills-ref` (`startswith` + `split("---", 2)`), and rule 6 diverges from
its `strip()`; both are recorded in ADR-001 §2.3. The trimmed body the client guide describes is
produced by `activation` (ADR-006), not stored, so that rendering keeps the body byte for byte
(ADR-005).

### 2.3 YAML subset (strictyaml parity)

The frontmatter is parsed as events. Every scalar becomes `RawValue::Str` whose `text` is the
scalar's text as written (unquoted and unescaped). No implicit typing happens anywhere.

`typed` records, without acting on it, what a typed YAML parser would do: it is `true` exactly
when the source scalar was plain (`ScalarStyle::Plain` on saphyr-parser's `Event::Scalar`) and
its text is empty or matches ADR-005 §2.3 row 6 (null, bool, int, float). `foo: true` and `foo:`
give `typed: true`; `foo: 'true'` and `foo: hello` give `typed: false`. Render uses it so that
an extension owned by another client keeps its YAML type (ADR-005). Spec fields and `metadata`
are strings by the spec and ignore it.

| # | Rule | Error | Test |
|---|---|---|---|
| 1 | Every scalar is a string: `version: 1.0` → `"1.0"`, `true` → `"true"`, `null` → `"null"`, `42` → `"42"` | — | `yaml_scalars_are_strings` |
| 2 | An empty value is the empty string | — | `empty_value_is_empty_string` |
| 3 | Flow collections (`[…]`, `{…}`) are rejected | `yaml-invalid` | `flow_collections_rejected` |
| 4 | Anchors, aliases and tags are rejected | `yaml-invalid` | `anchors_aliases_tags_rejected` |
| 5 | Duplicate keys in any mapping are rejected | `yaml-invalid` | `duplicate_keys_rejected` |
| 6 | A mapping key that is not a scalar is rejected | `yaml-invalid` | `non_scalar_key_rejected` |
| 7 | The root must be a mapping: empty, comment-only, scalar or sequence → error | `frontmatter-not-mapping` | `non_mapping_frontmatter_rejected` |
| 8 | Block scalars (`|`, `>`) keep the YAML-defined value | — | `block_scalars_follow_yaml` |
| 9 | Unknown top-level keys are kept in `entries`, in source order | — | `unknown_keys_retained_in_source_order` |
| 10 | `allowed-tools` is kept as one string | — | `allowed_tools_kept_as_string` |
| 11 | No input panics | — | `parse_never_panics` |
| 12 | `typed` is `true` only for plain scalars that are empty or match ADR-005 row 6 | — | `typed_flag_marks_plain_non_string_scalars` |
| 13 | Input after the root mapping's document (a second document, or invalid YAML after `...`) is rejected, as `strictyaml` rejects multi-document input | `yaml-invalid` | `input_after_the_first_document_is_rejected` |
| 14 | Parse time is linear in the input size | — | `deeply_repeated_collections_parse_in_linear_time` |
| 15 | Nesting deeper than 64 levels is rejected (bounds recursion on hostile input) | `yaml-invalid` | `nesting_deeper_than_64_levels_rejected` |

The `yaml-invalid` message is `Invalid YAML in frontmatter: {detail}`. The prefix matches
`skills-ref`; the detail text is this crate's own (ADR-001 §2.3).

### 2.4 Colon fallback

Only when the first YAML parse fails, retry once after rewriting every line that:

- has no leading whitespace and matches `^([A-Za-z0-9_-]+):[ \t]+(.+)$`;
- has a value that does not start with `"`, `'`, `|`, `>`, `[`, `{`, `&`, `*`, `!`, `#`;
- has a value that contains `": "` or ends with `:`.

The value (trailing whitespace removed) is replaced by a single-quoted scalar, with each `'`
doubled. If the retry parses, `RawSkill.repaired = Some(first error detail)`; otherwise the first
error is returned. Strict validation reports a repaired document as `yaml-invalid`; lenient load
reports `yaml-repaired` and continues (ADR-004).

| Rule | Test |
|---|---|
| `description: Use this skill when: the user asks about PDFs` is repaired | `colon_fallback_repairs_unquoted_colon` |
| Valid YAML is never rewritten (`repaired == None`) | `colon_fallback_never_applies_to_valid_yaml` |
| Indented lines are never rewritten | `colon_fallback_top_level_only` |
| A trailing `# comment` stays a comment: it never becomes part of a repaired value | `colon_fallback_leaves_commented_lines_alone` |

### 2.5 YAML backend (D7)

Evidence from `cargo info` and the crates.io API on 2026-11-06, plus sources in the local cargo
registry:

| Crate | Version | Status | Normal deps | `unsafe` | Implicit typing | Verdict |
|---|---|---|---|---|---|---|
| `serde_yaml` | 0.9.34+deprecated | deprecated (version suffix), last release 2024-03-25 | `unsafe-libyaml` and others | via `unsafe-libyaml` | serde-driven, YAML 1.1 | rejected: deprecated |
| `serde_norway` | 0.9.42 | fork, last release 2024-12-21 | `indexmap`, `itoa`, `ryu`, `serde`, `unsafe-libyaml-norway` | via `unsafe-libyaml-norway` (libyaml translated to Rust) | serde-driven | rejected: C-translated `unsafe` core, serde typing |
| `serde_yml` | 0.0.13 | description: "DEPRECATED — `serde_yml` is unmaintained", shim over `noyalib` | `noyalib`, `serde` | — | via `noyalib` | rejected: deprecated |
| `yaml-rust2` | 0.13.0 | active, 2026-09-11 | `arraydeque`, `hashlink`, `encoding_rs` | none in `src/` | loader types plain scalars | viable; carries a loader and `hashlink` this crate does not use |
| `saphyr` | 0.3.3 | active, 2026-10-27 | `saphyr-parser`, `hashlink`, `ordered-float`, `thiserror`, `encoding_rs` | none in `src/` | yes: `Scalar::parse_from_cow_and_metadata` resolves plain scalars to `Null`/`Boolean`/`Integer`/`FloatingPoint` | rejected as the loader: typing must be undone |
| **`saphyr-parser`** | **0.3.3** | active, 2026-10-27, 10.3M downloads | `arraydeque`, `thiserror` | none in `src/` | none: `Event::Scalar(Cow<str>, ScalarStyle, anchor_id, Option<Tag>)` is raw text | **chosen** |

`saphyr-parser` is the event layer under `saphyr` (D7's prior). Its event API exposes everything
§2.3 needs: scalars as raw text with style, `anchor_id` and tag (rules 1, 4); `Event::Alias`
(rule 4); `Span` with `Marker` positions, so a collection whose start is `[` or `{` is a flow
collection (rule 3). Duplicate keys and non-scalar keys (rules 5, 6) are checked while building
`RawValue`. Exactly one YAML crate appears in `cargo tree -p turul-agent-skills`.

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| `skills-ref` `split("---", 2)` | A valid quoted value containing `---` ends the block early |
| Trim the stored body (client guide) | Rendering could not keep the body byte for byte; `activation` trims instead |
| Accept flow style, anchors, tags | `skills-ref` rejects them; accepting them makes `validate` pass files the reference fails |
| Typed YAML (`saphyr`, serde) then stringify | Float formatting loses the text (`1.0` vs `1.00`); strictyaml keeps the text |
| Colon fallback in strict validation | `skills-ref` reports the YAML error; strict must match |
| Separate parse entrypoints for strict and lenient | Second parser; one entrypoint reports `repaired` and the callers decide |

## 4. Consequences

- New workspace dependency: `saphyr-parser = "0.3"` (0.3.3 evaluated; transitive `arraydeque`, `thiserror`).
- `SkillProperties.metadata` is a `BTreeMap`, so keys iterate sorted. `skills-ref`
  `read-properties` keeps source order; recorded in ADR-001 §2.3.
- Files `skills-ref` rejects for YAML reasons are rejected here too, except where §2.2 fixes the
  delimiter.

## 5. Revision log

- 2026-11-06 — Initial. strictyaml 1.7.3 probed; `saphyr-parser` 0.3.3 chosen; exact-line delimiters; verbatim body; top-level colon fallback.
- 2026-11-06 — Dependency stated as a major.minor workspace requirement; evaluation table re-checked with `cargo search` (versions unchanged).
- 2026-11-06 — Task-plan ids replaced by the command they named (one YAML crate in `cargo tree -p turul-agent-skills`).
- 2026-11-06 — `RawValue::Str` gains `typed` (plain source scalar that a typed parser reads as null/bool/int/float), so rendering keeps extension values such as `disable-model-invocation: true` unquoted.
- 2026-11-06 — Rules 13 and 14 added: the parser drains the stream and rejects anything after the first document; flow detection uses a precomputed character-to-byte index instead of a scan per collection. Colon fallback keeps trailing comments outside the quotes.
- 2026-11-06 — Accepted: `frontmatter.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only).
- 2026-11-06 — Rule 15: nesting deeper than 64 levels is `yaml-invalid` (was implemented, now stated and tested).
