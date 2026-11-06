# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow SemVer.

## [Unreleased]

## [0.1.0]

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
