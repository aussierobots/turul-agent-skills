# ADR-004: Strict validation and lenient loading

- **Status:** Accepted (2026-11-06; contract tests `validate_and_load.rs`, `conformance.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/validate_and_load.rs`, `crates/turul-agent-skills/tests/conformance.rs`
- **Depends on:** ADR-001 (corpus), ADR-002 (`SkillName`), ADR-003 (`parse_skill_md`); D4, D5

## 1. Context

`skills-ref` has two readers. `validator.py::validate` returns every error as a list of
messages. `parser.py::read_properties` raises on the first problem and ignores everything it
does not need. The client guide ("Lenient validation") asks a client to warn and still load on
name/directory mismatch and over-long names, and to skip a skill whose description is missing
or whose YAML is unparseable.

This crate needs both, with one vocabulary of errors, one field check, and no second parser.

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Error vocabulary | `ErrorCode` (`#[non_exhaustive]`, `as_str()` = stable kebab code) | — | `src/error.rs` |
| One problem | `ValidationError { code: ErrorCode, message: String }`; accessors `code()`, `message()`; `Display` = message | built only inside the crate | `src/error.rs` |
| Strict result | `Vec<ValidationError>` (empty = valid) | `validate(&Path) -> Vec<ValidationError>` | `src/validate.rs` |
| Loaded skill | `Skill { pub properties: SkillProperties, extensions: Vec<(String, RawValue)>, pub body: String, location: Option<PathBuf> }`; `Skill::new(SkillProperties, impl Into<String>)` (no extensions); `extensions() -> &[(String, RawValue)]`; `location() -> Option<&Path>` | `load` or `Skill::new` | `src/skill.rs` |
| Lenient result | `Loaded { skill: Skill, diagnostics: Vec<ValidationError> }`, `LoadSkip { errors: Vec<ValidationError> }` | `load(&Path) -> Result<Loaded, LoadSkip>` | `src/load.rs` |
| File lookup | private `find_skill_md(&Path) -> Option<PathBuf>` | used by `validate`, `load`, `discover` | `src/load.rs` |
| Field check | private `check(&RawSkill, dir_name: &str) -> Checked` | used by `validate` and `load` only | `src/validate.rs` |
| Message path text | public `display_path(&Path) -> String` (the CLI prints the same path text): components joined by `/`, `.` components dropped, empty → `.` (Python `pathlib` `str()`) | used by every message with a path | `src/error.rs` |

`validate` and `load` both run `find_skill_md` → read → `parse_skill_md` → `check`. They
differ only in how they classify the same `ValidationError`s (§2.4). `Skill`'s `PartialEq`
compares `properties`, `extensions` and `body`; `location` is provenance, not content.
`extensions` is private and read-only: only `load` fills it, from keys `parse_skill_md` found,
so an extension key is never a spec field name or a duplicate (ADR-003 rule 5).

`load` sets `location` to `canonicalize(dir)` joined with the file name found (absolute,
symlinks resolved, as `skills-ref` `to_prompt` does with `Path.resolve()`).

### 2.2 Order

`validate` stops at the first path or parse error (as `skills-ref` returns `[str(e)]`).
Otherwise it returns every field error in this order, which is `validate_metadata`'s order:
unknown keys, `name`, `description`, `compatibility`; then this crate's type checks for
`license`, `metadata`, `allowed-tools`. Within `name`: empty, length, the ADR-002 violations in
their order, directory match.

### 2.3 Error codes and messages

`{path}` = `display_path` of the path given; `{file}` = the file found; `{name}` = the
normalized name; `{dir}` = the final path component as given (`""` when there is none);
`{n}` = length in Unicode scalar values. "Source" names the `skills-ref` function that emits the
same text; "ours" marks a check `skills-ref` does not make (each is a row in ADR-001 §2.3).

| # | Code | Message (`validate`) | Source | `load` |
|---|---|---|---|---|
| 1 | `path-not-found` | `Path does not exist: {path}` | `validate` | n/a (reported as #3) |
| 2 | `not-a-directory` | `Not a directory: {path}` | `validate` | n/a (reported as #3) |
| 3 | `skill-md-missing` | `Missing required file: SKILL.md` | `validate` | skip; message `SKILL.md not found in {path}` (`read_properties`) |
| 4 | `read-failed` | `Failed to read {file}: {io error}` | ours (`skills-ref` raises an uncaught exception) | skip |
| 5 | `frontmatter-missing` | `SKILL.md must start with YAML frontmatter (---)` | `parse_frontmatter` | skip |
| 6 | `frontmatter-unclosed` | `SKILL.md frontmatter not properly closed with ---` | `parse_frontmatter` | skip |
| 7 | `yaml-invalid` | `Invalid YAML in frontmatter: {detail}` | `parse_frontmatter` (detail differs) | skip, unless repaired → #28 |
| 8 | `frontmatter-not-mapping` | `SKILL.md frontmatter must be a YAML mapping` | `parse_frontmatter` | skip |
| 9 | `unexpected-fields` | `Unexpected fields in frontmatter: {k1, k2, …}. Only ['allowed-tools', 'compatibility', 'description', 'license', 'metadata', 'name'] are allowed.` (keys sorted, joined by `, `) | `_validate_metadata_fields` | replaced by #29 per key |
| 10 | `name-missing` | `Missing required field in frontmatter: name` | `validate_metadata` | skip |
| 11 | `name-empty` | `Field 'name' must be a non-empty string` (also when not a scalar) | `_validate_name` | skip |
| 12 | `name-too-long` | `Skill name '{name}' exceeds 64 character limit ({n} chars)` | `_validate_name` | diagnostic |
| 13 | `name-not-lowercase` | `Skill name '{name}' must be lowercase` | `_validate_name` | skip |
| 14 | `name-hyphen-edge` | `Skill name cannot start or end with a hyphen` | `_validate_name` | skip |
| 15 | `name-consecutive-hyphens` | `Skill name cannot contain consecutive hyphens` | `_validate_name` | skip |
| 16 | `name-invalid-chars` | `Skill name '{name}' contains invalid characters. Only letters, digits, and hyphens are allowed.` | `_validate_name` | skip |
| 17 | `name-dir-mismatch` | `Directory name '{dir}' must match skill name '{name}'` (compares NFKC of both) | `_validate_name` | diagnostic |
| 18 | `description-missing` | `Missing required field in frontmatter: description` | `validate_metadata` | skip |
| 19 | `description-empty` | `Field 'description' must be a non-empty string` (also when not a scalar or only whitespace) | `_validate_description` | skip |
| 20 | `description-too-long` | `Description exceeds 1024 character limit ({n} chars)` (`n` of the untrimmed value) | `_validate_description` | diagnostic |
| 21 | `compatibility-not-string` | `Field 'compatibility' must be a string` | `_validate_compatibility` | diagnostic; field dropped |
| 22 | `compatibility-empty` | `Field 'compatibility' must be a non-empty string` | ours (spec: "Must be 1-500 characters if provided") | diagnostic; field dropped |
| 23 | `compatibility-too-long` | `Compatibility exceeds 500 character limit ({n} chars)` | `_validate_compatibility` | diagnostic; kept |
| 24 | `license-not-string` | `Field 'license' must be a string` | ours | diagnostic; field dropped |
| 25 | `metadata-not-mapping` | `Field 'metadata' must be a mapping of string keys to string values` | ours (spec: "a map from string keys to string values") | diagnostic; field dropped |
| 26 | `metadata-value-not-string` | `Field 'metadata' value for key '{key}' must be a string` | ours (`skills-ref` applies `str(v)`) | diagnostic; key dropped |
| 27 | `allowed-tools-not-string` | `Field 'allowed-tools' must be a string` | ours (spec: "space-separated string") | diagnostic; field dropped |
| 28 | `yaml-repaired` | `Frontmatter YAML was repaired by quoting values that contain ': ' ({detail})` | ours (client guide fallback) | diagnostic (load only) |
| 29 | `unknown-field` | `Unknown frontmatter field '{key}' kept as an extension` | ours (D4) | diagnostic (load only), one per key |

Codes 30–33 (`skill-shadowed`, `scan-limit-reached`, `symlink-escape`, `scan-io`) are owned by
ADR-009.

Stored values: `name` = normalized `SkillName`; `description` = trimmed (as
`read_properties`); `license`, `compatibility`, `allowed-tools` as parsed; an empty plain
`metadata:` value is an empty mapping (`skills-ref` accepts it).

### 2.4 Strict versus lenient

| Condition | `validate` | `load` |
|---|---|---|
| Codes 1–8 (except repaired YAML) | that single error | `Err(LoadSkip)` |
| Repaired YAML | `yaml-invalid` only (no field checks, as `skills-ref`) | `yaml-repaired` diagnostic, then field checks |
| Unknown top-level key (D4) | one `unexpected-fields` error | one `unknown-field` diagnostic per key; key and value kept in `Skill.extensions` in source order |
| Codes 10, 11, 13–16, 18, 19 | error | `Err(LoadSkip)` carrying every error found |
| Codes 12, 17, 20–27 | error | diagnostic; loading continues |

`LoadSkip.errors` lists every error found with the skip causes first, in `skills-ref` `read_properties` order: name-missing, description-missing, name-empty, description-empty, then the name identity rules, then the rest in report order. `errors[0]` is the reason the skill was skipped, and the CLI prints it (`skip_carries_every_error_with_the_skip_cause_first`, `skip_reason_is_the_error_that_caused_the_skip`).
`SkillProperties` holds spec fields only (D4); nothing outside the spec is ever typed.

### 2.5 File lookup (D5)

`find_skill_md(dir)` returns `dir/SKILL.md` if it exists, else `dir/skill.md` if it exists,
else `None` (`parser.py::find_skill_md`). Writes always produce `SKILL.md` (ADR-005).

### 2.6 Rules and tests

| Rule | Test |
|---|---|
| Every code in §2.3 has the exact message text | `messages_match_reference_text` |
| `ErrorCode::as_str` values are stable and unique | `error_codes_are_stable` |
| All field errors returned, in §2.2 order | `validate_reports_all_errors_in_reference_order` |
| Path and parse errors stop validation | `validate_stops_at_first_path_or_parse_error` |
| D4 strict: unknown keys → one sorted `unexpected-fields` | `unknown_key_is_strict_error_single_message_sorted` |
| D4 lenient: one `unknown-field` per key; kept in `extensions` in source order | `unknown_key_is_lenient_diagnostic_per_key_and_kept_in_extensions` |
| D4: `SkillProperties` has spec fields only | `skill_properties_fields_are_spec_fields` (ADR-002 routing file) |
| D5: `SKILL.md` preferred; `skill.md` accepted | `lowercase_skill_md_accepted_uppercase_preferred` |
| Length > 64: strict error, lenient diagnostic | `name_too_long_strict_error_lenient_diagnostic` |
| Directory mismatch: strict error, lenient diagnostic; NFKC on both sides | `dir_mismatch_strict_error_lenient_diagnostic`, `dir_match_uses_nfkc` |
| Missing or empty description → skip | `missing_description_skips` |
| Unparseable YAML → skip; repaired YAML → strict error, lenient diagnostic | `unparseable_yaml_skips`, `repaired_yaml_strict_error_lenient_diagnostic` |
| Identity-rule failures → skip | `invalid_name_skips` |
| Non-string optional fields: strict error, lenient drop + diagnostic | `non_string_optional_fields_strict_error_lenient_drop` |
| Empty `compatibility` rejected | `compatibility_empty_rejected` |
| A `SKILL.md` that is not UTF-8: `read-failed` from `validate` (stops, as a path error) and from `load` (skip) | `read_failed_on_non_utf8_skill_md` |
| `location` is canonical and absolute | `load_location_is_canonical_absolute` |
| No input panics `validate` or `load` | `validate_and_load_never_panic` |
| Corpus: each ADR-001 case gives its expected codes and load outcome | `corpus_cases_match_expected` (`conformance.rs`) |

### 2.7 House fixtures

`crates/turul-agent-skills/tests/fixtures/<id>/`, laid out as ADR-001 §2.2 (`<id>/<dir>/SKILL.md`
when a directory is named). `load`: ok = no diagnostics, warn = loaded
with diagnostics, skip = `LoadSkip`.

| id | dir | input | `validate` codes | `load` |
|---|---|---|---|---|
| house-dashes-in-quoted-value | my-skill | `description: "a --- b"` | — | ok |
| house-version-float | my-skill | `metadata: {version: 1.0}` (block style) | — | ok, `"1.0"` |
| house-int-metadata | my-skill | `metadata: {count: 42}` (block style) | — | ok, `"42"` |
| house-crlf | my-skill | CRLF line endings | — | ok, body keeps CRLF |
| house-no-trailing-newline | my-skill | file ends with `---` | — | ok, body `""` |
| house-lowercase-skill-md | my-skill | only `skill.md` | — | ok |
| house-unknown-key | my-skill | `disable-model-invocation: true` | `unexpected-fields` | warn (`unknown-field`); extension kept, `typed: true` |
| house-empty-extension | my-skill | `foo:` (empty unknown key) | `unexpected-fields` | warn (`unknown-field`); extension `""`, `typed: true` |
| house-colon-in-value | my-skill | `description: Use when: the user asks` (unquoted) | `yaml-invalid` | warn (`yaml-repaired`) |
| house-unicode-name | हिन्दी | `name: हिन्दी` (U+093F, U+094D, U+0940 are marks) | `name-invalid-chars` | skip |
| house-name-too-long | a×65 | `name:` a×65 | `name-too-long` | warn |
| house-dir-mismatch | wrong-name | `name: right-name` | `name-dir-mismatch` | warn |
| house-a2a-migrated | greet (ADR-008) | migrated A2A skill | — | ok |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Separate strict and lenient checkers | Two owners of the same rules; drift between them |
| Free-text errors only | Messages are parity text and may change upstream; codes are the contract |
| `load` stops at the first error | Callers surfacing diagnostics want everything at once |
| Accept names that fail identity rules in `load` | The client guide's note says the lenient approach "deliberately relaxes" the name's "matching the parent directory, character set, max length"; its bullet list names only directory match and length. This crate relaxes those two and skips on the character-set, case and hyphen rules, because `Skill` holds a `SkillName` (every module keys on it, ADR-002). Recorded as a divergence from the client guide in ADR-001 §2.3 |
| Coerce non-string values with `str()` as `skills-ref` does | Python `repr` text has no Rust equivalent; the spec says string values |

## 4. Consequences

- The CLI prints `ValidationError` messages and owns no wording (ADR-007).
- `read-properties` and `to-prompt` exit 1 on names `skills-ref` would print (uppercase, invalid
  characters, hyphen rules). Divergence taken by default pending maintainer review, ADR-001 §2.3.
- Callers that need in-memory validation write the document to a directory first; there is one
  validation entrypoint. The `author` example builds a `Skill` from drafted fields with
  `Skill::new`, writes it with `write_to`, then runs `validate` on that directory.
- The CLI also rejects a document `load` repaired with the colon fallback (ADR-007); `load` itself keeps it.

## 5. Revision log

- 2026-11-06 — Initial. 29 codes mapped to every `skills-ref` validator and parser message plus this crate's own checks; strict/lenient matrix; house fixtures.
- 2026-11-06 — Messages re-checked against `skills-ref` 0.1 `validator.py`, `parser.py`, `errors.py`: every message mapped. Fixture layout reference added.
- 2026-11-06 — Alternatives row restated without a task-plan id.
- 2026-11-06 — Client-guide quote on lenient names corrected (character set is named; this crate still skips, divergence recorded). `Skill.extensions` private with `extensions()` accessor. House fixtures `house-empty-extension`, `house-colon-in-value` added. `author` flow stated.
- 2026-11-06 — Test `read_failed_on_non_utf8_skill_md` added for the `read-failed` divergence.
- 2026-11-06 — `display_path` made public so the CLI reuses the one path-text owner.
- 2026-11-06 — Default taken pending maintainer review: lenient names skip (divergence in ADR-001). Colon fallback stays in `load`; the CLI no longer prints repaired documents (ADR-007).
- 2026-11-06 — `LoadSkip.errors` orders skip causes first so `errors[0]` is the skip reason.
- 2026-11-06 — Accepted: `validate_and_load.rs`, `conformance.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only). Defaults marked pending maintainer review stay marked; reversing one is a revision, not a new path.
