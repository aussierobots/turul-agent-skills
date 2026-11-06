# CLAUDE.md

1. Don’t assume. Don’t hide confusion. Surface tradeoffs.
2. Minimum code that solves the problem. Nothing speculative.
3. Touch only what you must. Clean up only your own mess.
4. Define success criteria. Loop until verified.

## Project Overview

Turul-agent-skills is a Rust implementation of the [Agent Skills](https://agentskills.io/specification) format (Linux Foundation / AAIF): parse, validate, render and catalog `SKILL.md` skill directories. Licensed under MIT OR Apache-2.0.

**This workspace owns the SKILL.md format.** Every other turul workspace that reads or writes SKILL.md (turul-a2a in particular) consumes this crate; none re-implements parsing, validation or rendering.

**Current release**: see `CHANGELOG.md`. Bootstrap plan: `docs/TASKS.md`.

## Build & Development Commands

```bash
cargo build --workspace
cargo test --workspace                     # No network required
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo doc --no-deps --workspace

cargo test -p turul-agent-skills           # Library: parse, validate, render, catalog
cargo test -p turul-agent-skills-cli       # skills-ref-compatible CLI
cargo test -p turul-agent-skills --test adr002_skill_name   # One ADR's contract tests

# Examples (offline stub LLM by default; live Ollama is opt-in)
cargo run -p skill-llm --bin catalog     # LLM picks a skill from the catalog, then uses it
cargo run -p skill-llm --bin author      # LLM drafts a SKILL.md; validate, write, modify
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin catalog
```

**All crate dependencies MUST use `workspace = true`** — versions live in root `[workspace.dependencies]`, dev-dependencies included. Never put a version number in a crate's own `Cargo.toml`.

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

- `turul-agent-skills` — library. `SkillName` (validated newtype, the only way to obtain a name), `SkillProperties` (frontmatter), `Skill` (properties + body + location), strict validation (spec, all errors), lenient loading (client guide, diagnostics), canonical rendering back to SKILL.md, `<available_skills>` catalog, bounded discovery over caller-supplied project/user roots (every candidate goes through the same loader). No LLM, network or async deps.
- `turul-agent-skills-cli` — `skills-ref`-compatible binary: `validate`, `read-properties`, `to-prompt`; exit codes 0/1.
- `examples/` — runnable examples that use skills with an LLM via [turul-llm](https://github.com/aussierobots/turul-llm) (`turul-llm-core` + an adapter). Offline stub by default.

### Dependency Direction (invariants)

- `turul-llm-*` is an **examples-only** dependency. It never appears in the `[dependencies]` of a publish crate.
- Publish crates do not depend on `turul-a2a` or any A2A type. A2A projection of a skill (e.g. to `AgentSkill`) lives in turul-a2a, built on this crate's types.
- `turul-agent-skills-cli` depends on `turul-agent-skills` and adds no format logic of its own.

### Normative Sources

- Specification: https://agentskills.io/specification
- Client implementation guide: https://agentskills.io/client-implementation/adding-skills-support
- Reference implementation `skills-ref` (Python): `github.com/agentskills/agentskills/skills-ref`, pinned by commit SHA in ADR-001. Its test corpus is ported to fixtures and is the conformance suite.

Where spec prose and `skills-ref` behaviour differ (name character set, `skill.md` fallback, YAML typing, empty catalog), the governing ADR records the decision and a test pins it. Re-verify the pinned SHA before describing a release as spec-compliant.

### Architecture Decision Records

Long-form rationale lives under `docs/adr/`. Each ADR names the owning symbol and entrypoint for every key and derived artifact, and stays Proposed until every alternative path is deleted. Each ADR has a contract test file `crates/<crate>/tests/adrNNN_<slug>.rs`. A contract change updates the ADR revision log, its tests and `CHANGELOG.md` in the same slice.

ADR refs belong in commit messages, PR descriptions, `CHANGELOG.md`, ADR cross-references and `README.md` files. **Not in code comments, docstrings or rustdoc.**

### ADR-review agent — mandatory precommit gate

Before committing any change touching `crates/`, `examples/`, `docs/adr/`, `CLAUDE.md`, `AGENTS.md`, `CHANGELOG.md` or the root `Cargo.toml`, dispatch the `adr-review` subagent (`.claude/agents/adr-review.md`). BLOCKERs and HIGHs are fixed before the commit lands.

### Comment and Docstring Style

Comments state the invariant, the contract, or the timeless "why". Never ADR numbers or sections, phase/slice/task labels, issue numbers or review history. Upstream spec citations (URL + field name) are fine when the comment would be incomplete without them.

### TDD Discipline

Tests are written from the spec, the `skills-ref` corpus and the ADR first; implementation follows. If code disagrees with a test, re-check the normative source before changing either. Only change a test when it is wrong relative to that source.

Contract tests prove: distinct inputs produce distinct identities; every caller obtains a `SkillName` through its constructor; rendering and catalog output are deterministic; parse ∘ render round-trips.

### Workspace Boundary

This workspace never edits turul-a2a, turul-llm or any sibling repo. Gaps found there go in a handoff naming the target session (owner, goal, gates, stop conditions).
