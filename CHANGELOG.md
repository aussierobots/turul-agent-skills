# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow SemVer.

## [Unreleased]

_Release 0.1.2 (`turul-agent-skills`, `turul-agent-skills-cli`; shared workspace version) when published._

### Changed

- Point package homepages and project links to Aussie Robots; add explicit API documentation links and review discovery metadata.
- Rewrite the `turul-agent-skills` docs.rs front page to open with a summary, one example and links onward.


## [0.1.1] - 2026-11-07

### Fixed

- The published `turul-agent-skills` package runs its own tests: fixtures are found inside the
  package, and the tests that read the rest of the workspace (examples, root README, every crate)
  are no longer shipped. `turul-agent-skills-cli` no longer ships its workspace-only CLI test.
  No library or CLI behaviour changed.

### Verification

`cargo test` in the extracted `turul-agent-skills-0.1.1.crate`: every shipped suite passes,
including the `skills-ref` conformance corpus.

## [0.1.0] - 2026-11-07

### Added

- `turul-agent-skills`: parse, validate, load, render and catalog Agent Skills (`SKILL.md`).
  `SkillName` (validated identity), `SkillProperties`, `Skill`; strict `validate` that reports
  every problem; lenient `load` that returns diagnostics and skips only unusable skills;
  canonical `write_to` that round-trips and preserves the Markdown body; `to_prompt` for the
  `<available_skills>` catalog; `activation` for a skill's instructions and bundled file list;
  `discover` over caller-supplied project and user directories.
- `turul-agent-skills-cli`: the `turul-skills` binary with `validate`, `read-properties` and
  `to-prompt`, matching `skills-ref` output and exit codes.
- `examples/skill-llm`: pick and activate a skill through an LLM (`catalog`), draft, write and
  modify a skill with one (`author`), and assert every public capability step by step (`smoke`).
  Offline stub by default; `RUN_OLLAMA_SMOKE=1` uses a live Ollama model. Three vendored
  Apache-2.0 reference skills from anthropics/skills live under `skills/reference`.
- `SkillBuilder` (`Skill::builder`): the construction entrypoint for new skills. `build` trims the
  description and returns `BuildError` (`Name`, `DescriptionEmpty`) for an illegal name or an empty
  description; the 1024-character limit stays a `validate` rule.
- Error types (`ParseError`, `ValidationError`, `NameError`, `BuildError`, `LoadSkip`) implement
  `std::error::Error` via `thiserror`; `ErrorCode` implements `Display`.
- `advise` and `MAX_RECOMMENDED_LINES`: authoring advisories for the two measurable specification
  recommendations, reported as `ErrorCode::SkillMdTooLong` (more than 500 rendered lines) and
  `ErrorCode::NestedReference` (a referenced file that references another file). Separate from
  `validate` and `load`; judgment of quality is the `skill-review` example's job.
- `examples/skill-llm`: the `skill-review` skill, a model of the best-practices page, and `smoke`
  steps `advise` and `llm-review-skill`.
- `load_str`: lenient loading from `SKILL.md` text, for skills embedded in a binary; the same
  rules as `load`, with the caller's directory name and no location.
- `examples/skill-llm`: `OLLAMA_NUM_CTX` sets the Ollama context window through
  `OllamaClient::with_num_ctx`, so the example needs a turul-llm 0.1 release that has it.
  Thinking models such as `gemma4` need `16384`; Ollama may default to 4096.
- `prelude` module for glob import of the common types and functions.
- README quick start whose snippets compile as doctests.

### Compatibility

- `NameViolation`, `NameError`, `BuildError`, `ScopeKind`, `Loaded`, `LoadSkip`, `Diagnostic`,
  `Discovered` and `RawSkill` are `#[non_exhaustive]`: fields can be read, not built by literal.
  `Scope` stays exhaustive.
- `SKILL.md` file size is not capped; the caller bounds what it scans.
- Follows the Agent Skills specification and agrees with `skills-ref` 0.1; all 40 of its test
  cases run as a conformance suite. Differences are listed in the README and recorded in
  ADR-001.
- Minimum supported Rust version 1.94. The publish crates have no async, network, LLM or A2A
  dependency.

### Verification

`cargo test --workspace` (no network), clippy with `-D warnings`, `cargo fmt --check`,
`cargo doc`, and `cargo package` for both publish crates.
