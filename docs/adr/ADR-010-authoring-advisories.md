# ADR-010: Authoring advisories

- **Status:** Accepted (2026-11-06; contract tests `advise.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/advise.rs`
- **Depends on:** ADR-001 (D18), ADR-004 (`ErrorCode`, `ValidationError`)

## 1. Context

The Agent Skills best-practices page and the specification's progressive-disclosure section
give guidance for writing skills. Two items are measurable: "keep your main SKILL.md under 500
lines" and "keep file references one level deep from SKILL.md". The rest (scope, gotchas,
defaults, what the agent already knows, description quality) is judgment. Validation (ADR-004)
checks the specification's rules; guidance is not a rule, so it must not fail `validate` or
`load`.

## 2. Decision

Best-practice guidance is not validation. A separate function reports only the two measurable
recommendations. Judgment is out of scope for the library and belongs to review: the
`skill-review` example skill (`examples/skill-llm/skills/skill-review`). Token counting is out of
scope (it needs a tokenizer).

### 2.1 Owners

| Artifact | Owner | Schema | Entrypoint |
|---|---|---|---|
| Advisory list | `advise(&Skill) -> Vec<ValidationError>` | `ValidationError` with `ErrorCode::SkillMdTooLong` or `ErrorCode::NestedReference` | `src/advise.rs`, re-exported at the crate root (not in the prelude) |
| Line limit | `pub const MAX_RECOMMENDED_LINES: usize = 500` | — | `src/advise.rs` |
| Reference bounds | `MAX_RESOURCE_FILES` (ADR-006), `SKIPPED_DIRS` (ADR-009) | — | reused, not redefined |

### 2.2 Rules

| Code | Condition | Message |
|---|---|---|
| `skill-md-too-long` | `skill.render()` has more than 500 lines | `SKILL.md has N lines; the specification recommends at most 500` |
| `nested-reference` | A file the body references (inside the skill directory) references another file inside the skill directory | `<first> references <second>; keep file references one level deep from SKILL.md` |

Reference detection: relative Markdown link targets `[text](path)` and single-backtick spans
that are a relative path, outside fenced code blocks. Ignored: URLs and any target containing
`:`, anchors (`#`), absolute paths, targets with a `..` component, targets that do not resolve
to an existing file, paths through `SKIPPED_DIRS`, and paths whose canonical form leaves the
canonical skill directory (so a symbolic link out of the skill is never read). Second-level
targets resolve against the referencing file's directory, then the skill directory. Only `.md`,
`.markdown` and `.txt` files are read, at most `MAX_RESOURCE_FILES` of them. A skill without a
`location()` gets only the line check.

Order is deterministic: the line check, then first-level files in order of first appearance in
the body, then targets in file order. Advisories never fail `validate` or `load` and are not
diagnostics of `load` (ADR-004).

## 3. Alternatives considered

| Alternative | Why not |
|---|---|
| Make the limits `validate` errors | Guidance, not specification; would reject valid skills |
| Count tokens (5,000-token recommendation) | Needs a tokenizer; the library has no model dependency |
| Judge description quality, gotchas, scope in the library | Judgment; belongs to review (`skill-review`) |
| A new return type for advisories | `ValidationError` and `ErrorCode` already are the vocabulary |

## 4. Consequences

- `ErrorCode` gains `skill-md-too-long` and `nested-reference`; `validate` and `load` never emit them.
- A reference that is also a first-level reference still counts as nested when another
  referenced file points at it; the advisory is a prompt to flatten, not a defect.
- Tests: `clean_skill_has_no_advisories`, `skill_md_over_500_lines_is_advised`,
  `skill_md_of_exactly_500_lines_is_not_advised`, `one_level_references_are_not_advised`,
  `reference_chain_is_advised_naming_both_files`, `urls_anchors_absolute_and_escaping_targets_are_ignored`,
  `skill_without_a_location_gets_only_the_line_check`, `advisories_are_deterministic_and_ordered`,
  `example_and_reference_skills_have_no_advisories`, `skill_review_passes_validate_and_advise`.

## 5. Revision log

- 2026-11-06 — Initial. `advise`, `MAX_RECOMMENDED_LINES`, two error codes; judgment left to `skill-review`.
- 2026-11-06 — Accepted: `advise.rs` green under `cargo test --workspace`.
