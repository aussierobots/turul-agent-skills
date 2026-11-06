# ADR-001: Spec sources and conformance corpus

- **Status:** Accepted (2026-11-06; contract tests `conformance.rs`, `dependency_direction.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/conformance.rs`, `crates/turul-agent-skills/tests/dependency_direction.rs`
- **Depends on:** nothing. Every later ADR cites this one for its normative source.

## 1. Context

The Agent Skills format is defined by prose on agentskills.io and by a Python reference
implementation, `skills-ref`. The prose leaves gaps (YAML typing, delimiter handling,
catalog escaping) and in one place reads two ways (the name character set). Without a
named source version and an executable corpus, "spec compliant" is an opinion.

Maintainer rule: **when in doubt, the reference sites resolve it.** Precedence: the
specification, then the client implementation guide, then `skills-ref` behaviour.

## 2. Decision

### 2.1 Sources, referenced by version

Sources are referenced by **version**, never by commit SHA or file hash.

| Source | Where | Version referenced |
|---|---|---|
| Specification | https://agentskills.io/specification (`docs/specification.mdx` in `github.com/agentskills/agentskills`) | Unversioned; the live page is the authority |
| Client guide | https://agentskills.io/client-implementation/adding-skills-support | Unversioned; the live page is the authority |
| Reference impl | `skills-ref` in `github.com/agentskills/agentskills` | `0.1` (`skills-ref/pyproject.toml`) |

Last read 2026-11-06. **Re-check:** before a release is called spec-compliant, re-read the
live spec and client guide, and the `skills-ref` tests on `main`. If the `skills-ref` version
moved past `0.1` or a test case changed, port the change and append a revision.

### 2.2 Corpus

`skills-ref` 0.1 `tests/` holds 40 pytest cases, none parametrized: `test_parser.py` 15,
`test_prompt.py` 4, `test_validator.py` 21. Each case is ported to
`crates/turul-agent-skills/tests/fixtures/<id>/` as data. No Python is copied into this
repository.

**Layout.** `<id>/expected.json` always. Files: `<id>/<dir>/SKILL.md` (or `skill.md`) when the
case names a directory, one directory per named skill; else `<id>/SKILL.md`. A case that git
cannot hold as files sets `layout` (below), and `conformance.rs` builds it in a fresh temp
directory `T` instead of using the fixture directory:

| `layout` | Case path | Built from | Cases |
|---|---|---|---|
| absent | `<id>/<dir>`, or `<id>` when `dir` is `—` | the fixture files | all others |
| `empty-dir` | `T/my-skill`, an empty directory (the name is never read: `skill-md-missing` comes first) | nothing | `p-missing-skill-md`, `p-find-none`, `v-missing-skill-md` |
| `missing` | `T/nonexistent`, not created | nothing | `v-nonexistent-path` |
| `regular-file` | `T/file.txt`, contents `test` | nothing | `v-not-a-directory` |
| `both-cases` | `T/my-skill` holding `SKILL.md` and `skill.md` | `<id>/my-skill/SKILL.md` and `<id>/lowercase.md` (one name per file, so a case-insensitive checkout holds both) | `p-prefer-uppercase` |

**`expected.json`.** One object; which fields are present depends on the case kind:

| Field | Type | Present for | Asserts |
|---|---|---|---|
| `layout` | string, table above | the six cases above | — |
| `parse` | `"ok"` or an ADR-004 code | P cases with `dir` `—` | `parse_skill_md(<id>/SKILL.md)` result or `ParseError::code()` |
| `entries` | object of strings | `p-valid-frontmatter` | these top-level entries of the parsed `RawSkill` |
| `body_contains` | string | `p-valid-frontmatter` | `RawSkill.body` contains it |
| `valid` | bool | every case with one case path | `validate(path).is_empty()` |
| `errors` | array of ADR-004 codes | same | codes of `validate(path)`, in order |
| `load` | `"ok"`, `"warn"` or `"skip"` | same | `load(path)`: no diagnostics, diagnostics, `LoadSkip` |
| `properties` | object | `p-read-valid`, `p-read-metadata`, `p-read-lowercase`, `p-allowed-tools`, `p-prefer-uppercase`, `house-version-float`, `house-int-metadata` | equals the ADR-007 §2.4 properties object of the loaded skill, key order included |
| `prompt` | `null` or object `{contains: [string], absent: [string], count: {string: int}}` | R cases | `to_prompt` over `load` of every directory under `<id>/` in byte order: `null` = returns `None`; else each `contains` substring present, each `absent` substring missing, each `count` substring occurs exactly that often |

R cases have no single case path; `r-single` and `r-escaping` also carry `valid`, `errors`,
`load`. P cases with `dir` `—` carry `parse` instead of `valid`, `errors`, `load` (the
`skills-ref` case is a string passed to `parse_frontmatter`, not a directory). House fixtures
use the same schema.

`p-prefer-uppercase` changes only file contents: `skills-ref` writes the bodies `uppercase` and
`lowercase` and inspects `find_skill_md`, which is private here (ADR-004 §2.5). The fixture's
`SKILL.md` has `description: uppercase` and `lowercase.md` has `description: lowercase`;
`properties.description` is `"uppercase"`.

The expected outcome below is the `skills-ref` assertion. ADR-004 maps each to a stable error
code; ADR-006 and ADR-007 pin byte-level prompt and CLI output.

Columns: `id` | `src` (P parser, V validator, R prompt) | `dir` (directory name; `—` = none) | `input` | `expect`.

| id | src | dir | input | expect |
|---|---|---|---|---|
| p-valid-frontmatter | P | — | name+description, body `# My Skill…` | name, description parsed; body contains `# My Skill` |
| p-missing-frontmatter | P | — | `# No frontmatter here` | parse error: must start with frontmatter |
| p-unclosed-frontmatter | P | — | opening `---`, no close | parse error: not properly closed |
| p-invalid-yaml | P | — | `name: [invalid` | parse error: invalid YAML |
| p-non-mapping | P | — | YAML list | parse error: must be a mapping |
| p-read-valid | P | my-skill | + `license: MIT` | properties name, description, license |
| p-read-metadata | P | my-skill | block `metadata:` with `author: Test Author`, `version: 1.0` | metadata `{"author":"Test Author","version":"1.0"}` (string, not float) |
| p-missing-skill-md | P | my-skill | empty directory (`skills-ref` uses `tmp_path` itself) | error: SKILL.md not found |
| p-missing-name | P | my-skill | description only | error: missing required field name |
| p-missing-description | P | my-skill | name only | error: missing required field description |
| p-prefer-uppercase | P | my-skill | both `SKILL.md` and `skill.md` | `SKILL.md` chosen |
| p-accept-lowercase | P | my-skill | only `skill.md` | `skill.md` found |
| p-find-none | P | my-skill | no file | not found |
| p-read-lowercase | P | my-skill | only `skill.md`, valid | properties read |
| p-allowed-tools | P | my-skill | `allowed-tools: Bash(jq:*) Bash(git:*)` | kept as the string; serialised key `allowed-tools` |
| v-valid | V | my-skill | minimal valid | no errors |
| v-nonexistent-path | V | — | path missing | 1 error: does not exist |
| v-not-a-directory | V | — | regular file | 1 error: not a directory |
| v-missing-skill-md | V | my-skill | no file | 1 error: missing required file SKILL.md |
| v-name-uppercase | V | MySkill | `name: MySkill` | error: lowercase |
| v-name-too-long | V | a×70 | name a×70 | error: exceeds 64 character limit |
| v-name-leading-hyphen | V | -my-skill | `-my-skill` | error: cannot start or end with a hyphen |
| v-name-double-hyphen | V | my--skill | `my--skill` | error: consecutive hyphens |
| v-name-invalid-chars | V | my_skill | `my_skill` | error: invalid characters |
| v-name-dir-mismatch | V | wrong-name | `correct-name` | error: directory must match skill name |
| v-unexpected-fields | V | my-skill | `unknown_field: …` | error: unexpected fields |
| v-all-fields | V | my-skill | license + metadata | no errors |
| v-allowed-tools | V | my-skill | allowed-tools | no errors |
| v-i18n-chinese | V | 技能 | `技能` | no errors |
| v-i18n-russian-hyphen | V | мой-навык | `мой-навык` | no errors |
| v-i18n-russian-lower | V | навык | `навык` | no errors |
| v-i18n-russian-upper | V | НАВЫК | `НАВЫК` | error: lowercase |
| v-description-too-long | V | my-skill | description x×1100 | error: exceeds 1024 |
| v-compatibility-valid | V | my-skill | `compatibility: Requires Python 3.11+` | no errors |
| v-compatibility-too-long | V | my-skill | x×550 | error: exceeds 500 |
| v-nfkc | V | café (U+00E9) | `cafe` + U+0301 | no errors (NFKC equal) |
| r-empty | R | — | `[]` | exactly `<available_skills>\n</available_skills>` |
| r-single | R | my-skill | minimal | contains `<name>\nmy-skill\n</name>`, `<description>\nA test skill\n</description>`, `<location>`, `SKILL.md` |
| r-multiple | R | skill-a, skill-b | two skills | two `<skill>` elements, both names present |
| r-escaping | R | special-skill | `Use <foo> & <bar> tags` | `&lt;foo&gt;`, `&amp;`, `&lt;bar&gt;`; no raw `<foo>` |

House fixtures (not from `skills-ref`) are listed in ADR-004 §2.7, beside the rules they
pin; `house-a2a-migrated` is specified in ADR-008. The corpus is 40 cases and the house list 13,
so `tests/fixtures/` holds 53 case directories. Goldens (`expected.render.md`,
`expected.prompt.xml`) and the directory layout are described in `tests/fixtures/README.md`.

### 2.3 Divergences from the reference sources

Each row has a decision and a test. A divergence never goes in silently. The second column is
`skills-ref` behaviour unless the row names the client guide.

Rows marked **Same** are recorded parity decisions, not divergences. The Test column names the
test that pins each row; the file is in parentheses.

| Topic | `skills-ref` | This crate | Why | Owner | Test |
|---|---|---|---|---|---|
| Name character set | NFKC, `isalnum()` or `-`, lowercase | **Same** (general category `L*`/`Nd`/`Nl`/`No`, proven equal to `isalnum` over all code points) | Spec prose says "unicode lowercase alphanumeric"; 5 corpus cases need Unicode | ADR-002 | `parse_accepts_unicode_lowercase` (`skill_name.rs`) |
| Unquoted `: ` in a value | `Invalid YAML in frontmatter` everywhere | Library `load` repairs it (client guide colon fallback) with a `yaml-repaired` diagnostic; `validate` and the CLI report it as `skills-ref` does | Client guide recommends the fallback for agent clients; CLI parity kept | ADR-003, ADR-007 | `repaired_yaml_strict_error_lenient_diagnostic` (`validate_and_load.rs`), `repaired_yaml_exit_1_like_skills_ref` (`cli.rs`) |
| Printed name | `read_properties` returns the stripped name without NFKC | NFKC form everywhere (`read-properties`, `to-prompt`) | One identity per name | ADR-002 | `printed_name_is_nfkc` (`cli.rs`) |
| Lowercase `skill.md` | Accepted on read; `SKILL.md` preferred | **Same**; writes always produce `SKILL.md` | Parity | ADR-004, ADR-005 | `lowercase_skill_md_accepted_uppercase_preferred` (`validate_and_load.rs`), `write_to_creates_name_dir_and_uppercase_file` (`render.rs`) |
| YAML subset | strictyaml: all scalars strings; flow, anchors, tags, duplicate keys rejected | **Same** | Parity | ADR-003 | `yaml_scalars_are_strings`, `flow_collections_rejected`, `anchors_aliases_tags_rejected`, `duplicate_keys_rejected` (`frontmatter.rs`) |
| Opening delimiter | `content.startswith("---")` (`---x` passes) | Line 1 is exactly `---` | A delimiter is a line | ADR-003 | `delimiter_first_line_must_be_exactly_dashes` (`frontmatter.rs`) |
| Closing delimiter | `content.split("---", 2)`: a `---` inside a value ends the block | First later line that is exactly `---` | A value containing `---` is valid YAML | ADR-003 | `dashes_inside_quoted_value_do_not_close` (`frontmatter.rs`) |
| Body | `parts[2].strip()` (not exposed by any `skills-ref` command) | Stored verbatim; `activation` trims | Render keeps the body byte for byte | ADR-003, ADR-006 | `body_preserved_verbatim` (`frontmatter.rs`), `activation_wraps_trimmed_body_with_directory` (`catalog_prompt.rs`) |
| YAML error text | `Invalid YAML in frontmatter: {strictyaml text}` | Same prefix, own detail text | Different parser | ADR-003 | `messages_match_reference_text` (`validate_and_load.rs`) |
| Optional field types | `license`, `allowed-tools`, non-mapping `metadata` unchecked; `metadata` values `str()`-coerced | `*-not-string`, `metadata-not-mapping`, `metadata-value-not-string` | Spec: strings; string-to-string map | ADR-004 | `non_string_optional_fields_strict_error_lenient_drop` (`validate_and_load.rs`) |
| Empty `compatibility` | Accepted | `compatibility-empty` | Spec: "Must be 1-500 characters if provided" | ADR-004 | `compatibility_empty_rejected` (`validate_and_load.rs`) |
| Unreadable or non-UTF-8 file | Uncaught exception, exit 1 | `read-failed`, exit 1 | Errors are values | ADR-004 | `read_failed_on_non_utf8_skill_md` (`validate_and_load.rs`) |
| Names failing identity rules in `read-properties` / `to-prompt` (default pending maintainer review) | Printed (no checks beyond non-empty) | Exit 1 with the rule's message | `Skill` holds a valid `SkillName` | ADR-004 | `invalid_name_skips` (`validate_and_load.rs`), `cli_matches_table_on_every_fixture` (`cli.rs`, `v-name-uppercase` loads `skip`) |
| Name character set, case, hyphens in lenient `load` / `discover` (default pending maintainer review) | Client guide ("Lenient validation" note): "deliberately relaxes" character set, as well as length and directory match | Length and directory match warn; the other name rules skip (`invalid_name_skips`) | `Skill` holds a `SkillName`, the key of catalog, activation and discovery | ADR-002, ADR-004 | `invalid_name_skips`, `name_too_long_strict_error_lenient_diagnostic`, `dir_mismatch_strict_error_lenient_diagnostic` (`validate_and_load.rs`) |
| `read-properties` metadata key order | Source order | Sorted (byte order) | `metadata` is a `BTreeMap`; render is sorted | ADR-003, ADR-005 | `read_properties_metadata_sorted_when_source_unsorted` (`cli.rs`) |
| Empty catalog | Library returns the empty wrapper; CLI cannot reach it (`required=True`, exit 2) | Library returns `None`; CLI exits 2 on no paths | Client guide: omit an empty catalog | ADR-006, ADR-007 | `to_prompt_empty_is_none` (`catalog_prompt.rs`), `to_prompt_without_paths_exit_2` (`cli.rs`) |
| `<location>` escaping | Not escaped | **Same** | Must stay a usable file path | ADR-006 | `to_prompt_location_not_escaped` (`catalog_prompt.rs`) |
| Usage, `--help`, `--version` text | click | Missing path: click-shaped block; other usage errors, `--help`, `--version`: clap. Same exit codes and final error line | Different CLI library | ADR-007 | `missing_path_exit_2` (`cli.rs`) |

### 2.4 Dependency direction (§deps)

Publish crates (`crates/*`) carry no normal or target-specific dependency whose name starts
with `turul-llm`, `turul-a2a`, `tokio` or `reqwest`. LLM clients are used only by
`examples/`. A2A projection lives in turul-a2a, on top of this crate's types.

Owner: `crates/turul-agent-skills/tests/dependency_direction.rs`, which parses every
`crates/*/Cargo.toml`:

| Rule | Test |
|---|---|
| No publish crate has a forbidden normal or target-specific dependency | `publish_crates_have_no_forbidden_dependencies` |
| The detector catches plain and target-specific entries | `detector_catches_plain_and_target_specific_dependencies` |

### 2.5 Examples and turul-llm (§examples)

The examples use `turul-llm-core` and `turul-llm-ollama` at `0.1` (crates.io). The
interface used, from `turul-llm-core` 0.1:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
}
// CompletionRequest::new(rendered_prompt).with_output_schema(serde_json::Value)
// CompletionResponse { parsed_output: serde_json::Value }
```

"Prompt plus JSON Schema returns a structured pick" is expressible as it stands: the catalog
goes into `rendered_prompt` and the choice is a schema `enum` of skill names. **No turul-llm
gap, so no turul-llm handoff.** The offline stub implements `LlmClient` in the example, as
`turul-llm-core`'s own tests do.

### 2.6 Maintainer decisions

The other ADRs cite these by label. Decided by the maintainer on 2026-11-06.

| # | Decision | Recommendation / decision |
|---|----------|----------------|
| D1 | Starting version | `0.1.0`; patch bumps thereafter |
| D2 | Licence | Dual `MIT OR Apache-2.0`: rename `LICENSE`→`LICENSE-APACHE`, add `LICENSE-MIT` |
| D3 | `name` charset: spec prose "unicode lowercase alphanumeric characters (`a-z`, `0-9`)" vs skills-ref NFKC+Unicode `isalnum` | **Revised 2026-11-06: follow skills-ref** (NFKC, Unicode lowercase alphanumeric + hyphen). Was ASCII; reversed under the maintainer rule "if in doubt the reference sites resolve": the prose is ambiguous (says "unicode") and 5 of 21 `skills-ref` validator cases (`test_i18n_*`, `test_nfkc_normalization`) require Unicode. No divergence |
| D4 | Dialects / unknown frontmatter keys | Spec is the only dialect modelled. Strict `validate`: unknown key = error. Lenient `load`: unknown top-level keys kept verbatim in opaque `extensions` (insertion order), one diagnostic per key; `render` writes them after spec fields. No typed vendor fields or per-dialect parsers, ever |
| D5 | `skill.md` lowercase fallback | Accept on read (skills-ref parity); `render`/`write_to` always write `SKILL.md` |
| D6 | `to_prompt` on empty list | Library returns `None` on empty; the CLI cannot reach the empty case (`to-prompt` requires a path, exit 2) — parity with skills-ref |
| D7 | YAML backend | `saphyr-parser` 0.3, chosen in ADR-003; every scalar read as a string |
| D8 | Crate count | 2 crates (lib + cli) — keeps `clap` out of the library |
| D9 | A2A extras | turul-a2a migrates to the spec: `name` replaces `id` (an `_` id becomes `-` in `name`; `metadata.a2a-id` keeps the wire id and `SkillCard` uses it as `AgentSkill.id` when present, ADR-008); scalar extras → namespaced `metadata` string keys (`a2a-tags`…); JSON Schemas → `assets/*.schema.json` referenced from metadata. turul-a2a's `SkillCard` is built from `Skill` and still projects to proto `AgentSkill`; its `SkillCard::parse` is deleted there |
| D10 | Discovery | **Build.** ADR-009: caller-supplied project/user roots; bounded scan; project beats user; every candidate through `load`; trust gating is the caller's concern. Sync `std::fs` |
| D11 | LLM examples | One member `examples/skill-llm`, bins `catalog`, `author` and `smoke`, shared stub client |
| D12 | CLI on unquoted `: ` values | `read-properties` / `to-prompt` fail like `skills-ref`; only the library's `load` repairs (ADR-003, ADR-007) |
| D13 | Lenient names | `load` warns on length and directory mismatch but skips names that break identity rules (ADR-002, ADR-004) |
| D14 | Construction | `SkillBuilder` (turul builder style) is the one public way to create a skill; `build` rejects an empty description (ADR-004) |
| D15 | Errors | `thiserror` derives, `#[non_exhaustive]` enums, as in turul-a2a and turul-mcp-framework |
| D16 | File size | Not capped; the caller bounds what it scans (ADR-004) |
| D17 | CI | GitHub Actions: fmt, clippy, test, doc on stable; `cargo check` on MSRV 1.94 |
| D18 | Best practices | `advise` reports the two measurable spec recommendations (ADR-010); judgment is the `skill-review` example's job |

## 3. Consequences

- `conformance.rs` is table-driven over `tests/fixtures/*/expected.json` (schema §2.2):

  | Rule | Test |
  |---|---|
  | Each of the 40 ids above has exactly one fixture directory, and every non-`house-` fixture is one of them | `every_corpus_id_has_one_fixture` |
  | Every fixture, corpus and house, meets each field of its `expected.json` | `corpus_cases_match_expected` |
- A behaviour change that moves a corpus outcome changes this table, the fixture and the
  owning ADR in the same commit.
- Moving to a new `skills-ref` version is a deliberate commit with a revision entry, never a drive-by.

## 4. Revision log

- 2026-11-06 — Initial. 40 cases; D3 follows `skills-ref` (Unicode, NFKC); turul-llm interface quoted.
- 2026-11-06 — Divergence table completed from ADR-002..007: location stays unescaped (parity); empty catalog unreachable from the `skills-ref` CLI; opening delimiter, body, type checks, printed name, metadata order, error text rows added.
- 2026-11-06 — Commit-SHA and file-hash pins removed; sources referenced by version (`skills-ref` 0.1, `turul-llm` 0.1, spec and client guide live). Maintainer rule: refer to versions, not SHAs. File renamed from `ADR-001-spec-pinning-and-conformance-corpus.md`.
- 2026-11-06 — Fixture layout stated (`<id>/<dir>/SKILL.md`), matching `house-a2a-migrated/greet/` (ADR-008). Remaining SHA and patch-version references removed from this log. Divergence table unchanged by the ADR-002..009 review.
- 2026-11-06 — Divergence table covers the client guide too: lenient name character set (guide relaxes, this crate skips); colon fallback in `read-properties` / `to-prompt` (`skills-ref` exits 1, this crate prints).
- 2026-11-06 — Divergence table gains a Test column, one test per row; three new tests named (`printed_name_is_nfkc`, `read_failed_on_non_utf8_skill_md`, `read_properties_metadata_sorted_when_source_unsorted`). `expected.json` schema and `layout` encoding for cases git cannot hold as files stated here; `conformance.rs` and `dependency_direction.rs` test names stated.
- 2026-11-06 — Fixtures ported (53 case directories). `properties` also asserted for `house-version-float` and `house-int-metadata`; fixture count and golden pointer stated.
- 2026-11-06 — Default pending maintainer review: the CLI does not apply the colon fallback (parity with `skills-ref`); its divergence row removed, the library keeps the fallback. Lenient name skipping recorded as a divergence.
- 2026-11-06 — Differential run against `skills-ref` 0.1: 144 comparisons (53 fixtures x `validate`, `read-properties`, `to-prompt`), exit code, stdout and stderr per command. 120 identical; the 24 mismatches before the colon-fallback change each map to a row above (closing delimiter, identity-rule names, YAML error detail, usage text for a leading-hyphen path, printed NFKC name) except `house-colon-in-value`, fixed by the maintainer decision above.
- 2026-11-06 — Divergence table re-verified: the current `turul-skills` re-run against all 144 recorded `skills-ref` 0.1 outputs gives 24 mismatches, each in a row above: YAML error detail 6 (`p-invalid-yaml`, `house-colon-in-value`, now exit 1 like `skills-ref`), closing delimiter 3, identity-rule names 10, usage text 3, printed NFKC name 2. The six `layout` cases and `r-empty` have no files for the differential; `conformance.rs` pins them to the `skills-ref` assertions.
- 2026-11-06 — Accepted: `conformance.rs`, `dependency_direction.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only). Defaults marked pending maintainer review stay marked; reversing one is a revision, not a new path.
- 2026-11-06 — Usage-text row: missing-path block is click-shaped, the rest is clap.
- 2026-11-06 — Divergence row for the colon fallback restored at library level: `load` repairs, `validate` and the CLI match `skills-ref`.
- 2026-11-06 — §2.6 holds the maintainer decision table (D1–D17), moved from the bootstrap plan, which is retired.
- 2026-11-06 — D18 added: best-practice guidance is advisory (`advise`, ADR-010), not validation.
