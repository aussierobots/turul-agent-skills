# CLAUDE.md

1. Don’t assume. Don’t hide confusion. Surface tradeoffs.
2. Minimum code that solves the problem. Nothing speculative.
3. Touch only what you must. Clean up only your own mess.
4. Define success criteria. Loop until verified.

## Project Overview

Turul-agent-skills is a Rust implementation of the [Agent Skills](https://agentskills.io/specification) format (Linux Foundation / AAIF): parse, validate, render and catalog `SKILL.md` skill directories. Licensed under MIT OR Apache-2.0.

**This workspace owns the SKILL.md format.** Every other turul workspace that reads or writes SKILL.md (turul-a2a in particular) consumes this crate; none re-implements parsing, validation or rendering.

**Current release**: see `CHANGELOG.md`. Bootstrap plan and post-bootstrap changes: `docs/TASKS.md`. CI (`.github/workflows/ci.yml`) runs fmt, clippy `-D warnings`, test and `cargo doc` with `RUSTDOCFLAGS=-D warnings` on stable, plus `cargo check` on the MSRV (1.94).

## Build & Development Commands

```bash
cargo build --workspace
cargo test --workspace                     # No network required
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo doc --no-deps --workspace

cargo test -p turul-agent-skills           # Library: parse, validate, render, catalog
cargo test -p turul-agent-skills-cli       # skills-ref-compatible CLI
cargo test -p turul-agent-skills --test conformance   # skills-ref corpus; others: skill_name, frontmatter,
#   validate_and_load, builder, render, catalog_prompt, discovery, a2a_boundary, single_owner,
#   dependency_direction, readme_snippets
cargo test -p turul-agent-skills-cli --test cli       # CLI output and exit codes
cargo test --doc --workspace               # README quick-start snippets (library README; root README copy checked by readme_snippets)
cargo run -p turul-agent-skills-cli -- validate path/to/skill   # binary: turul-skills

# Examples (offline stub LLM by default; live Ollama is opt-in)
cargo run -p skill-llm --bin catalog     # LLM picks a skill from the catalog, then uses it
cargo run -p skill-llm --bin author      # LLM drafts a SKILL.md; validate, write, modify
cargo run -p skill-llm --bin smoke       # every public capability, asserted step by step
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin catalog
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin smoke
```

**All crate dependencies MUST use `workspace = true`** — versions live in root `[workspace.dependencies]`, dev-dependencies included. Never put a version number in a crate's own `Cargo.toml`.

**Versions, not SHAs.** Upstream sources (spec, `skills-ref`) and sibling crates (turul-llm, turul-a2a) are referenced by version, never by commit SHA or file hash. Sibling crates are major.minor only (`turul-llm-core = "0.1"`, not `0.1.1`); this workspace's own release number stays whole. `Cargo.lock` is git-ignored and gates nothing.

**Parallel worktrees:** give each worktree its own `CARGO_TARGET_DIR`.

## Git Conventions

- Commit messages: succinct, no Co-Authored-By or any AI attribution.
- Do not push unless explicitly asked.
- Patch bumps by default; see `CHANGELOG.md` for per-release classification. Every behaviour or contract change bumps the version and updates `CHANGELOG.md` in the same commit.

## CHANGELOG.md style

For adopters and maintainers: what changed for someone consuming these crates. Use `### Added` / `### Changed` / `### Fixed` / `### Removed` / `### Compatibility`, plus a one-line `### Verification` when behaviour-sensitive. No task/phase numbers, commit hashes, ADR section numbers or review-loop narration. `see ADR-00N` is fine.

## Release & Publish (crates.io)

Publishing is irreversible. Never run `cargo publish` without explicit instruction; authorize each call separately.

1. Bump `[workspace.package].version` and the matching `version` on intra-workspace `[workspace.dependencies]` entries.
2. Write the CHANGELOG entry.
3. Gate: test, clippy `-D warnings`, fmt, doc, and `cargo package -p turul-agent-skills -p turul-agent-skills-cli --no-verify --allow-dirty` all clean.
4. Commit, push `main`, tag `vX.Y.Z`, push the tag.
5. Publish `turul-agent-skills`, then `turul-agent-skills-cli`.

Every publish crate carries `LICENSE-APACHE` and `LICENSE-MIT` symlinks to `../../LICENSE-*`.

## Architecture

### Crate Structure

- `turul-agent-skills` — library. `SkillName` (validated newtype, the only way to obtain a name), `SkillProperties` (frontmatter), `Skill` (properties + body + location), `SkillBuilder` (`Skill::builder`; the construction entrypoint for new skills, `BuildError` on an illegal name or empty description), error types derived with `thiserror`, a `prelude` module for glob import, strict validation (spec, all errors), lenient loading (client guide, diagnostics), canonical rendering back to SKILL.md, `<available_skills>` catalog, bounded discovery over caller-supplied project/user roots (every candidate goes through the same loader). No LLM, network or async deps.
- `turul-agent-skills-cli` — `skills-ref`-compatible binary `turul-skills`: `validate`, `read-properties`, `to-prompt`; exit codes 0/1, 2 for a missing path or usage error.
- `examples/skill-llm` — runnable examples (bins `catalog`, `author`, `smoke`) that use skills with an LLM via [turul-llm](https://github.com/aussierobots/turul-llm) (`turul-llm-core` + `turul-llm-ollama`). Offline stub by default; `RUN_OLLAMA_SMOKE=1` (and `OLLAMA_MODEL`) selects live Ollama. `skills/reference` holds three vendored Apache-2.0 skills from anthropics/skills.

### Dependency Direction (invariants)

- `turul-llm-*` is an **examples-only** dependency. It never appears in the `[dependencies]` of a publish crate.
- Publish crates do not depend on `turul-a2a` or any A2A type. A2A projection of a skill (e.g. to `AgentSkill`) lives in turul-a2a, built on this crate's types.
- `turul-agent-skills-cli` depends on `turul-agent-skills` and adds no format logic of its own.

### Normative Sources

- Specification: https://agentskills.io/specification
- Client implementation guide: https://agentskills.io/client-implementation/adding-skills-support
- Reference implementation `skills-ref` (Python): `github.com/agentskills/agentskills/skills-ref`, referenced by version (0.1) in ADR-001 — never by commit SHA. Its test corpus is ported to fixtures and is the conformance suite.

Where spec prose and `skills-ref` behaviour differ (name character set, `skill.md` fallback, YAML typing, empty catalog), the governing ADR records the decision and a test pins it. Re-check the live spec and the current `skills-ref` version before describing a release as spec-compliant.

### Architecture Decision Records

Long-form rationale lives under `docs/adr/`. Each ADR names the owning symbol and entrypoint for every key and derived artifact, and stays Proposed until every alternative path is deleted. Each ADR names its contract test file(s) in its `Implements:` line. Test files, modules and functions are named for the behaviour they test (`tests/skill_name.rs`), never after an ADR. A contract change updates the ADR revision log, its tests and `CHANGELOG.md` in the same slice.

ADR refs belong in commit messages, PR descriptions, `CHANGELOG.md`, ADR cross-references and `README.md` files. **Not in code comments, docstrings or rustdoc.**

### ADR-review agent — mandatory precommit gate

Before committing any change touching `crates/`, `examples/`, `docs/adr/`, `CLAUDE.md`, `AGENTS.md`, `CHANGELOG.md` or the root `Cargo.toml`, dispatch the `adr-review` subagent (`.claude/agents/adr-review.md`). BLOCKERs and HIGHs are fixed before the commit lands.

### Naming, Comment and Docstring Style

**Names say what a thing is.** No file, module, test, function, type or fixture is named after an ADR, task or phase (`adr001_corpus.rs`, `t3_4_test`). Name it for its behaviour (`tests/conformance.rs`). The ADR points at the code; the code never points at the ADR.

Comments state the invariant, the contract, or the timeless "why". Never ADR numbers or sections, phase/slice/task labels, issue numbers or review history. Upstream spec citations (URL + field name) are fine when the comment would be incomplete without them.

### TDD Discipline

Tests are written from the spec, the `skills-ref` corpus and the ADR first; implementation follows. If code disagrees with a test, re-check the normative source before changing either. Only change a test when it is wrong relative to that source.

Contract tests prove: distinct inputs produce distinct identities; every caller obtains a `SkillName` through its constructor; rendering and catalog output are deterministic; parse ∘ render round-trips.

### Workspace Boundary

This workspace never edits turul-a2a, turul-llm or any sibling repo. Gaps found there go in a handoff naming the target session (owner, goal, gates, stop conditions).
