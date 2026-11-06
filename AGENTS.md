# Repository Guidelines

## Current State

- 0.1.0 implemented: `turul-agent-skills` (parse, validate, load, `SkillBuilder`, render, catalog, activation, discover, `prelude`) and `turul-agent-skills-cli` (binary `turul-skills`: `validate`, `read-properties`, `to-prompt`). Contract tests in `crates/*/tests/` (`conformance`, `skill_name`, `frontmatter`, `validate_and_load`, `builder`, `render`, `catalog_prompt`, `discovery`, `a2a_boundary`, `single_owner`, `dependency_direction`, `readme_snippets`, `cli`). Maintainer decisions: ADR-001 §2.6. CI: `.github/workflows/ci.yml`.
- Target: the [Agent Skills specification](https://agentskills.io/specification) (Linux Foundation / AAIF), with the `skills-ref` reference implementation as conformance oracle.
- Surface: the two crates above and `examples/skill-llm` (`cargo run -p skill-llm --bin catalog` / `--bin author` / `--bin smoke`; offline stub, `RUN_OLLAMA_SMOKE=1` for live Ollama) using [turul-llm](https://github.com/aussierobots/turul-llm).
- Primary downstream consumer: [turul-a2a](https://github.com/aussierobots/turul-a2a), which will create and modify SKILL.md through this crate.

## Source of Truth

In this order:

1. The Agent Skills specification at `https://agentskills.io/specification`.
2. The client implementation guide at `https://agentskills.io/client-implementation/adding-skills-support`.
3. `skills-ref` at `github.com/agentskills/agentskills/skills-ref`, at the version named in ADR-001 (0.1).
4. Accepted and Proposed ADRs under `docs/adr/`.
5. This file, then `CLAUDE.md`.
6. Patterns from turul-a2a and turul-mcp-framework.

When in doubt, the reference sites (1–3) resolve it — never local preference or memory. Fetch and cite them.

If spec prose and `skills-ref` disagree, neither wins silently: the governing ADR records the decision and a test pins it.

## Default Role In This Repo

- Act as critic and architecture reviewer first: spec compliance, single ownership of the format, boundary discipline, test quality.
- Do not make implementation changes unless explicitly asked.
- For review requests, lead with concrete findings.

## Response Style

- Answer the direct question first. Shortest answer that keeps the key caveat or next action.
- Separate confirmed facts from assumptions; say what was verified and when.
- For go/no-go, state the decision, then only blocking issues.

## Non-Negotiable Format Rules

### One Owner

- This workspace is the only implementation of SKILL.md parsing, validation, rendering and catalog output across the turul workspaces. A parallel implementation elsewhere is drift; it is reported and migrated, not tolerated.
- `SkillName` is built only by its constructor. No caller compares, normalises or validates names from raw strings.
- One parser, one validator, one renderer, one catalog producer. Discovery and the CLI route through the loader; no second path "for convenience".

### Spec Truthfulness

- Strict validation reports every spec violation, not the first one.
- Lenient loading follows the client guide: warn and load on cosmetic issues; skip when the description is missing or the YAML is unparseable; always return diagnostics.
- Unknown frontmatter keys, YAML typing and the name character set are decided in ADRs, not ad hoc.
- Rendering is canonical and deterministic; parse ∘ render round-trips and the Markdown body is preserved byte-for-byte.

### Version Hygiene

- Reference sources by version, never by commit SHA or file hash: `skills-ref` version and spec read date in ADR-001. Sibling crates (turul-llm, turul-a2a) are referenced as major.minor (`0.1`), never the patch.
- `Cargo.lock` is git-ignored and gates nothing.
- Patch bumps by default. Every behaviour or contract change bumps the version and updates `CHANGELOG.md` in the same commit.

## Architecture Guardrails

### Keep Core Publishable

- `turul-agent-skills` has no network, async, LLM or A2A dependency.
- `turul-llm-*` is examples-only; it never enters a publish crate's `[dependencies]`.
- The CLI adds argument handling and exit codes only; all format logic lives in the library.
- A2A projection (`AgentSkill` and A2A-specific extras) lives in turul-a2a, on top of this crate's types. How those extras fit the spec's string→string `metadata` is ADR-governed here.

### Examples

- Offline stub `LlmClient` by default; live providers are opt-in via environment variables. `cargo test --workspace` never needs a network.
- Examples use the public library API only. Needing an internal helper means the API is missing something; design it.

## Testing Policy

### Test-First

- Tests come from the spec, the ported `skills-ref` corpus and the ADR, before implementation.
- Each ADR names its contract test file(s) in its `Implements:` line; those tests are the executable contract. Nothing in code (file, module, test, function, fixture) is named after an ADR: name it for what it is (`tests/skill_name.rs`).

### Required Test Layers

- Name rules: valid/invalid corpus, directory match, distinct names stay distinct.
- Parsing: delimiters, malformed YAML fallback, metadata typing.
- Validation: strict all-errors and lenient diagnostics, each mapped to a corpus fixture.
- Rendering: determinism, field order, quoting, round-trip.
- Catalog: byte-exact output against `skills-ref to-prompt` fixtures, escaping, empty-set behaviour.
- Discovery: routes through `load`, skip/cap rules, project-over-user collisions with diagnostics, deterministic order.
- CLI: subcommands, output and exit codes match `skills-ref`.
- README: the library README's snippets compile as doctests; the project README's Rust snippets must equal library snippets (`readme_snippets.rs`).

### Compliance Audit Gate

Before describing a release as spec-compliant: live spec re-read and `skills-ref` version re-checked, corpus fixtures re-ported if it changed, and every divergence listed in an ADR with its test.

## Review Discipline

- Check ADR drift: the code's public API matches the owning symbols each ADR names.
- Check dependency direction and that no publish crate gained an LLM, async or A2A dep.
- Check for a second implementation of anything the library owns.

## Planning Rules

- ADR before implementation for any contract. An ADR stays Proposed until every alternative path is deleted.
- No new feature, crate or infrastructure without the maintainer's explicit yes. State the one-sentence ask and what breaks without it.
- Delete superseded paths in the same slice, callers and tests included. No compatibility shims.

## Workspace Boundary

- Never edit turul-a2a, turul-llm or any sibling repo from here. Report gaps as a handoff naming the target session: owner, goal, gates, stop conditions.
- Commits carry no AI attribution.
