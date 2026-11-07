# ADR-008: A2A projection boundary and ownership migration

- **Status:** Proposed (stays Proposed until turul-a2a deletes `SkillCard::parse`)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/a2a_boundary.rs`
- **Depends on:** ADR-001 (§deps), ADR-003, ADR-004, ADR-005; D4, D9

## 1. Context

turul-a2a 0.2 has its own `SKILL.md` parser:
`crates/turul-a2a-patterns/src/manifest.rs::SkillCard::parse`. Its private `Frontmatter` is
`#[serde(rename_all = "camelCase", deny_unknown_fields)]`, requires `id`, `name`,
`description`, parses with `serde_yaml` 0.9 (deprecated, ADR-003), and splits frontmatter on
`\n---`. `SkillCard::to_agent_skill` projects eight fields onto proto `AgentSkill`.

Two parsers for one file format means two owners. They already disagree:

| Direction | What breaks today |
|---|---|
| a2a file → this crate / `skills-ref` | `tags: [demo, …]` flow sequences are `yaml-invalid` (strictyaml rejects flow); after that, `unexpected-fields` for `id`, `tags`, `examples`, `inputModes`, `outputModes`, `securityRequirements`, `inputSchema`, `outputSchema`, `executionHints`, `providerConfig`; `name: Greet` fails `name-not-lowercase`; directory `demo` ≠ name |
| spec file → `SkillCard::parse` | missing `id` is an error; `license`, `compatibility`, `metadata`, `allowed-tools` are rejected by `deny_unknown_fields` |
| a2a `id` → spec `name` | `check_invariants`, `validate_against_schema` (`agent-role-critic-agent`) and `echo_loud` (`skill-dispatch-profile-agent`) contain `_`, which fails `name-invalid-chars` (`skills-ref` `_validate_name`: `c.isalnum() or c == "-"`); interop clients and smoke tests send these ids on the wire |

D9: turul-a2a moves to the spec. This crate stays A2A-free (ADR-001 §deps).

## 2. Decision

### 2.1 Owners

| Artifact | Owner | Entrypoint |
|---|---|---|
| `SKILL.md` parse, validation, load | this crate | `turul_agent_skills::load` / `validate`; `load_str` for a skill embedded in a binary |
| `SkillCard` and its projection to `AgentSkill` | turul-a2a | `SkillCard: TryFrom<&Skill>`, `SkillCard::to_agent_skill` (unchanged) |
| `a2a-*` metadata keys and `assets/*.json` contents | turul-a2a | read through `Skill.properties.metadata` and the skill directory (`Skill::location`), or for an embedded skill through an a2a-supplied asset reader keyed by the same relative path |

This crate's public API names no A2A type, key or concept.

### 2.2 Field mapping (D9)

Every `SkillCard` field and every `ExecutionHints` field. List values are a JSON array of
strings stored as one metadata string (compact, no spaces); an empty list omits the key.
Paths are relative to the skill directory (spec "File references").

| `SkillCard` field | a2a frontmatter today | Spec home | Encoding |
|---|---|---|---|
| `id` | `id` | `name` | `SkillName`; the directory is renamed to match. `AgentSkill.id` = `metadata.a2a-id` if present, else `name` |
| `name` | `name` (display text) | `metadata.a2a-name` | string; absent → `AgentSkill.name` = spec `name` |
| `description` | `description` | `description` | unchanged |
| `tags` | `tags` | `metadata.a2a-tags` | JSON array |
| `examples` | `examples` | `metadata.a2a-examples` | JSON array |
| `input_modes` | `inputModes` | `metadata.a2a-input-modes` | JSON array |
| `output_modes` | `outputModes` | `metadata.a2a-output-modes` | JSON array |
| `security_requirements` | `securityRequirements` | `metadata.a2a-security-requirements` | JSON array of scheme names |
| `input_schema` | `inputSchema` | `assets/input.schema.json` | file; `metadata.a2a-input-schema` = its path |
| `output_schema` | `outputSchema` | `assets/output.schema.json` | file; `metadata.a2a-output-schema` = its path |
| `execution_hints.max_tokens` | `executionHints.maxTokens` | `metadata.a2a-max-tokens` | decimal string |
| `execution_hints.temperature` | `executionHints.temperature` | `metadata.a2a-temperature` | decimal string |
| `execution_hints.top_p` | `executionHints.topP` | `metadata.a2a-top-p` | decimal string |
| `provider_config` | `providerConfig` | `assets/provider-config.json` | file; `metadata.a2a-provider-config` = its path |
| `body` | body | body | verbatim |

An `id` that is not a valid `SkillName` becomes the spec `name` with each `_` replaced by `-`
(`check_invariants` → `check-invariants`, `validate_against_schema` → `validate-against-schema`,
`echo_loud` → `echo-loud`), and `metadata.a2a-id` keeps the original wire id
(`a2a-id: check_invariants`). turul-a2a's `SkillCard` uses `a2a-id` as `AgentSkill.id` when
present and `name` otherwise, so the wire ids and every caller that sends them (the agents'
`src/main.rs` dispatch, their `tests/smoke.rs`, `remote-delegate-agent`, the Go, Python and Rust
clients under `examples/interop-clients/{agent-role-critic,skill-dispatch-profile}/`) are
unchanged. The catalog identity is `name`; the wire identity is `a2a-id` or, absent that, `name`.
Default taken pending maintainer review (2026-11-06).

Nothing is dropped. Values become strings because the spec's `metadata` is string to string;
nested objects become files because the spec puts data files under `assets/`.

### 2.3 Migrated fixture

`skill-manifest-ollama-agent/skills/demo/SKILL.md` migrated by §2.2 lives at
`crates/turul-agent-skills/tests/fixtures/house-a2a-migrated/greet/`. The case directory holds
the skill directory because a strict-valid skill's directory must equal its name (`greet`).
Metadata is written in `render` order, so the file is also its own render golden. Body is
byte-identical to the source example.

The source directory is `demo`; the fixture directory is `greet` (= `id` = spec `name`), and the
case's `expected.json` (`{"valid":true,"errors":[],"load":"ok"}`) sits beside it at
`house-a2a-migrated/expected.json` (ADR-001 §2.2).
`securityRequirements: []` is empty, so `a2a-security-requirements` is omitted. JSON Schemas and
provider config are the source values as JSON files under `assets/`.

strictyaml 1.7.3 parses the frontmatter (probe, as in ADR-003) to the expected string map.

### 2.4 Rules and tests

| Rule | Test |
|---|---|
| Migrated fixture passes strict `validate` | `migrated_a2a_fixture_is_strict_valid` |
| It loads with no diagnostics; `render` equals the file bytes | `migrated_a2a_fixture_loads_clean_and_renders_identically` |
| Every metadata key starts with `a2a-` | `migrated_a2a_fixture_metadata_keys_are_namespaced` |
| Every path-valued key names an existing file in the skill directory | `migrated_a2a_fixture_referenced_assets_exist` |
| No identifier or string containing `a2a` (any case) in `crates/turul-agent-skills/src/` | `public_api_has_no_a2a_symbol` |
| No `turul-a2a*` dependency | `dependency_direction.rs` (ADR-001) |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Sidecar `skill.a2a.json` beside `SKILL.md` | Two files describe one skill and drift; other clients ignore it anyway |
| Typed A2A fields on `SkillProperties` | D4: one dialect, spec fields only |
| Extension trait in this crate (`trait SkillDialect`) | Puts dialect hooks in the format owner; still a second schema |
| Keep `SkillCard::parse` alongside | Two parsers, two owners, already incompatible (§1) |
| Nested YAML maps under `metadata` | Spec: string values only; `validate` rejects them (`metadata-value-not-string`) |
| Space-separated lists | `examples` contain spaces and commas; JSON arrays are unambiguous |
| Rename the wire id to the hyphenated `name` | Breaks the three wire ids and every caller that sends them, including interop clients outside the slice's control |

## 4. Consequences

- turul-a2a gains a dependency on this crate and loses `serde_yaml`.
- Every a2a example directory is renamed to its spec `name` (§2.2).
- Wire ids `check_invariants`, `validate_against_schema`, `echo_loud` are unchanged; they live in
  `metadata.a2a-id` and the skill `name` is the hyphenated form. A skill has two identities
  (catalog `name`, wire `a2a-id`) that turul-a2a's `SkillCard` keeps apart.
- This ADR becomes Accepted only when turul-a2a has deleted `SkillCard::parse`.

## 5. Handoff — turul-a2a session (not executed here)

- Owner: turul-a2a session. Goal: replace `crates/turul-a2a-patterns/src/manifest.rs::SkillCard::parse` with `turul_agent_skills::load` + `SkillCard: TryFrom<&Skill>` reading namespaced `metadata` keys and `assets/*.json` files per ADR-008; `SkillCard::to_agent_skill` unchanged; migrate every example `SKILL.md` to spec frontmatter (`name` = dir name, drop `id`); `_` ids → `-` for `name` with the original id in `metadata.a2a-id`, and `SkillCard` projects `a2a-id` (else `name`) to `AgentSkill.id`, per §2.2; delete `serde_yaml` from the a2a workspace.
- Gates: `turul-skills validate` exits 0 on every a2a example skill; a2a tests and interop smoke green; `rg serde_yaml` and wire ids unchanged (interop clients and smoke tests untouched); a2a ADR revision log updated; patch bump + CHANGELOG by its owner.
- Stop: any A2A field without an ADR-008 row, or any `id` that is not a valid `SkillName` after `_` → `-` → report back, do not invent a mapping; any need to change this crate's API → report, do not fork.

## 6. Revision log

- 2026-11-06 — Initial. Full `SkillCard` field mapping; migrated `greet` fixture; handoff.
- 2026-11-06 — turul-a2a referenced by version (0.2), not commit; fixture layout and `demo` → `greet` rename stated; handoff names `assets/*.json` (provider config is not a schema).
- 2026-11-06 — Ids that fail `SkillName` (`check_invariants`, `validate_against_schema`, `echo_loud`): `_` → `-`, wire id changes with listed callers; `a2a-id` rejected; handoff gate and stop added. `expected.json` content stated and added to the fixture.
- 2026-11-06 — Default pending maintainer review: non-`SkillName` ids map to the hyphenated `name` plus `metadata.a2a-id` holding the wire id; `SkillCard` uses `a2a-id` as `AgentSkill.id` when present. Wire ids and their callers are unchanged; the rename alternative is rejected, the `a2a-id` alternative adopted.
- 2026-11-06 — Re-checked at release: stays Proposed, turul-a2a still defines `SkillCard::parse`; `a2a_boundary.rs` green.
- 2026-11-06 — `id` row aligned with the `a2a-id` rule below it (`AgentSkill.id` = `a2a-id` when present).
- 2026-11-07 — Embedded skills: turul-a2a loads `include_str!` manifests with `load_str`; asset files of an embedded skill are read through an a2a-owned reader keyed by the `assets/` relative path; `TryFrom<&Skill>` stays the disk-backed form. No new field mapping.
