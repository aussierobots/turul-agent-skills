# turul-agent-skills — bootstrap task list

Goal: a Rust workspace that is the single owner of the Agent Skills `SKILL.md` format
(https://agentskills.io/specification): parse, strict-validate, lenient-load, render, catalog-prompt, discover,
plus a `skills-ref`-compatible CLI. ADR-first, TDD, conformance corpus ported from the reference impl.
Layout mirrors `/home/nick/turul-a2a` (resolver 3, edition 2024, rust-version 1.94, all deps `workspace = true`,
dual MIT/Apache with per-crate symlinks, `crates/<name>/{src,tests,README.md}`, `docs/adr/`, `CHANGELOG.md`,
`.claude/agents/adr-review.md`). No ADR/phase/task refs in code comments. No AI attribution anywhere.

## Success criteria (whole bootstrap)
- `cargo test --workspace` green; every ADR-001..009 names its contract test files (named for the behaviour, never the ADR) and they pass.
- Every collected pytest item in `skills-ref/tests/test_{parser,prompt,validator}.py` (`skills-ref` 0.1) ported as a fixture;
  CLI exit codes/stdout match `skills-ref` on all of them except divergences listed in ADR-001.
- `load(render(skill)) == skill` on every valid fixture; body bytes preserved; `render` and `to_prompt` deterministic.
- No second SKILL.md parser in this workspace: `validate`, `load`, `discover` all route through `parse_skill_md`.
- `turul-llm-*` deps appear only under `examples/`; `cargo test --workspace` passes with no network.
- Root README quick start compiles as doctests.
- clippy `-D warnings`, `fmt --check`, `doc --no-deps`, `cargo package` clean. `0.1.0` tagged only after P5.
- turul-a2a handoff written in ADR-008 (not executed here).

## Decisions
All decided by maintainer 2026-11-06 (recommendation accepted; D10 = build).
| # | Decision | Recommendation / decision |
|---|----------|----------------|
| D1 | Starting version | `0.1.0`; patch bumps thereafter |
| D2 | Licence | Dual `MIT OR Apache-2.0`: rename `LICENSE`→`LICENSE-APACHE`, add `LICENSE-MIT` |
| D3 | `name` charset: spec prose "unicode lowercase alphanumeric characters (`a-z`, `0-9`)" vs skills-ref NFKC+Unicode `isalnum` | **Revised 2026-11-06: follow skills-ref** (NFKC, Unicode lowercase alphanumeric + hyphen). Was ASCII; reversed under the maintainer rule "if in doubt the reference sites resolve": the prose is ambiguous (says "unicode") and 5 of 21 `skills-ref` validator cases (`test_i18n_*`, `test_nfkc_normalization`) require Unicode. No divergence |
| D4 | Dialects / unknown frontmatter keys | Spec is the only dialect modelled. Strict `validate`: unknown key = error. Lenient `load`: unknown top-level keys kept verbatim in opaque `extensions` (insertion order), one diagnostic per key; `render` writes them after spec fields. No typed vendor fields or per-dialect parsers, ever |
| D5 | `skill.md` lowercase fallback | Accept on read (skills-ref parity); `render`/`write_to` always write `SKILL.md` |
| D6 | `to_prompt` on empty list | Library returns `None` on empty; the CLI cannot reach the empty case (`to-prompt` requires a path, exit 2) — parity with skills-ref |
| D7 | YAML backend | Evaluated in T1.5 from docs.rs/`cargo info`; prior `saphyr`; scalars read as strings |
| D8 | Crate count | 2 crates (lib + cli) — keeps `clap` out of the library |
| D9 | A2A extras | turul-a2a migrates to the spec: `name` replaces `id` (an `_` id becomes `-` in `name`; `metadata.a2a-id` keeps the wire id and `SkillCard` uses it as `AgentSkill.id` when present, ADR-008); scalar extras → namespaced `metadata` string keys (`a2a-tags`…); JSON Schemas → `assets/*.schema.json` referenced from metadata. turul-a2a's `SkillCard` is built from `Skill` and still projects to proto `AgentSkill`; its `SkillCard::parse` is deleted there |
| D10 | Discovery | **Build.** ADR-009: caller-supplied project/user roots; bounded scan; project beats user; every candidate through `load`; trust gating is the caller's concern. Sync `std::fs` |
| D11 | LLM examples | One member `examples/skill-llm`, bins `catalog` + `author`, shared stub client |

Model tiers: H haiku, S sonnet, O opus. Durable output from H/S is opus-checked before commit (T3.17, T5.4).
No P1–P3 output (ADRs, fixtures, code) is committed before T3.17; fixtures from P2 are verified by T3.1 green + T4.3 differential first.

## P0 — Scaffold
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T0.1 | Root `Cargo.toml`: members `crates/turul-agent-skills`, `crates/turul-agent-skills-cli`; `[workspace.package] version="0.1.0"`, `edition="2024"`, `rust-version="1.94"`, `resolver="3"`, license, repository | `Cargo.toml` | `cargo metadata --no-deps` lists both — **done 2026-11-06** | H | D1, D8 |
| T0.2 | `docs/adr/`, `CHANGELOG.md` (`## [Unreleased]`) | same | files exist — **done 2026-11-06** | H | — |
| T0.3 | Licences per D2 | `LICENSE-APACHE`, `LICENSE-MIT` | both present — **done 2026-11-06** | H | D2 |
| T0.4 | Crate stubs `crates/turul-agent-skills/{src/lib.rs,tests/,README.md}`, `crates/turul-agent-skills-cli/{src/main.rs,tests/,README.md}`; per-crate `LICENSE-*` symlinks → `../../LICENSE-*` | crate dirs | `cargo check --workspace`; 4 symlinks resolve — **done 2026-11-06** | H | T0.1, T0.3 |
| T0.5 | Port `.claude/agents/adr-review.md` from turul-a2a for this workspace: comment-style grep, workspace-dep rule, one-owner rule, symlinks, no-attribution, dep direction (`turul-llm-*` only in `examples/*/Cargo.toml`; no async/network/A2A dep in publish crates) | `.claude/agents/adr-review.md` | "No findings." on the stub tree — **done 2026-11-06** | S | T0.4 |
| T0.6 | `tests/dependency_direction.rs`: parse `crates/*/Cargo.toml`, assert no `turul-llm*`, `turul-a2a*`, `tokio`, `reqwest` in `[dependencies]` | test | green; sabotage: add dep → red → revert — **done 2026-11-06** | S | T0.4 |
| T0.7 | Clone agentskills/agentskills (`main`, `skills-ref` 0.1) into the session scratchpad (untrusted; no `.py` copied into this repo). Count collected cases per file: parse `def test_` + expand `@pytest.mark.parametrize` (or `pytest --collect-only -q` in an isolated venv) | scratchpad | per-file counts recorded for T1.1 — **done 2026-11-06: parser 15, prompt 4, validator 21 = 40; no parametrize** | H | — |

## P1 — ADRs (Proposed; each names owning symbol + entrypoint, lists its test file, has a revision log)
Header format: `turul-a2a/docs/adr/ADR-024-typed-skill-handler.md` lines 1–8.
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T1.1 | ADR-001 spec sources & conformance corpus: spec + client-guide URL and date read; `skills-ref` version (0.1) — versions only, never SHA or hash; case table with expected outcome; divergence table (D6 library side, delimiter rule, location escaping if changed). Spec, client guide and skills-ref all live in agentskills/agentskills; sections **§deps** (examples-only `turul-llm`, no async/A2A in publish crates → `dependency_direction.rs`) and **§examples**; contract files `conformance.rs`, `dependency_direction.rs` | `docs/adr/ADR-001-spec-sources-and-conformance-corpus.md` | version recorded; every case in table — **drafted 2026-11-06 (40 cases)**; **Accepted 2026-11-06** | O | T0.7 |
| T1.2 | Read `turul-llm` `examples/greet-ollama` + `LlmClient` signature for turul-llm 0.1; quote it in ADR-001 §examples. If it cannot express "prompt + JSON Schema → structured pick", fill the turul-llm handoff (bottom) into §examples; else delete that block | ADR-001, this file | signature quoted for 0.1; handoff present iff gap named — **done: turul-llm 0.1, no gap, handoff removed** | O | T1.1 |
| T1.3 | ADR-002 `SkillName`: sole constructor `SkillName::parse(&str)`; enforces non-empty, charset (D3), no leading/trailing/double hyphen. **Length ≤64 and parent-dir match are validation rules (ADR-004), not identity rules**, so lenient load can warn and still hold a `SkillName`. `Display`, `Eq`, `Hash`, `Ord`; private field; no `From<String>`. Contract file `skill_name.rs`, `single_owner.rs` | `docs/adr/ADR-002-skill-name-identity.md` | every rule has a test name — **drafted 2026-11-06**; **Accepted 2026-11-06** | O | D3 |
| T1.4 | ADR-003 frontmatter parsing & YAML backend: evaluate `serde_yaml` (deprecated), `serde_norway`, `serde_yml`, `saphyr`, `yaml-rust2` on implicit typing (`version: 1.0` stays `"1.0"`), metadata coercion, maintenance, `unsafe`, dep weight — from docs.rs/`cargo info`, not memory. Delimiter rule: line 1 exactly `---`, closing `---` on its own line (divergence from skills-ref `split("---",2)`). Owning symbols `SkillProperties`, `RawSkill`; entrypoint `parse_skill_md(&str) -> Result<RawSkill, ParseError>`; colon fallback for malformed YAML lives here | `docs/adr/ADR-003-frontmatter-parsing.md` | eval table filled; D7 library chosen — **drafted 2026-11-06: `saphyr-parser` 0.3**; **Accepted 2026-11-06** | O | D4, D7 |
| T1.5 | ADR-004 validation vs loading: `validate(&Path) -> Vec<ValidationError>` (all errors, stable order + codes; unknown key, length >64, dir mismatch = errors); `load(&Path) -> Result<Loaded{skill, diagnostics}, LoadSkip>` (length/dir mismatch/unknown key → diagnostic, unknown keys into `Skill.extensions`; missing description / unparseable YAML after fallback → skip); `skill.md` fallback per D5; D4 invariant: `SkillProperties` spec fields only. One error vocabulary; CLI prints from it | `docs/adr/ADR-004-strict-validation-and-lenient-loading.md` | every skills-ref message mapped; D4/D5 have test names — **drafted 2026-11-06 (29 codes)**; **Accepted 2026-11-06** | O | T1.3, T1.4, D4, D5 |
| T1.6 | ADR-005 rendering: `Skill::render() -> String`, `Skill::write_to(parent) -> Result<PathBuf>` (dir = name, file `SKILL.md`); field order `name, description, license, compatibility, metadata, allowed-tools`, then `extensions` in insertion order; metadata keys sorted; quoting table (`:`, `#`, leading/trailing space, `---`, non-ASCII); body bytes verbatim; `load(render(skill)) == skill` incl. extensions | `docs/adr/ADR-005-rendering.md` | quoting table complete; round-trip rule stated — **drafted 2026-11-06**; **Accepted 2026-11-06** | O | T1.5 |
| T1.7 | ADR-006 catalog prompt: `to_prompt(&[Skill]) -> Option<String>` (D6); name/description escaped as Python `html.escape` (`& < > " '`); `<location>` parity (skills-ref does not escape) or escape + divergence; skills-ref byte layout; input order preserved; `activation(&Skill) -> String` `<skill_content>` wrapper with resource listing (not read) | `docs/adr/ADR-006-catalog-prompt.md` | expected bytes for 0/1/2 skills; D6 recorded — **drafted 2026-11-06 (`<location>` unescaped, parity)**; **Accepted 2026-11-06** | O | T1.5, D6 |
| T1.8 | ADR-007 CLI parity: **binary name fixed here** (recommend `skills-ref`-style `turul-skills`); `validate <path>`, `read-properties <path>`, `to-prompt <path>...`; exit 0/1; stdout/stderr split; `read-properties` JSON key order; accepts path to `SKILL.md` file or dir | `docs/adr/ADR-007-cli-parity.md` | invocation → exit/stdout table — **drafted 2026-11-06: `turul-skills`; empty wrapper unreachable (exit 2)**; **Accepted 2026-11-06** | O | T1.5, T1.7 |
| T1.9 | ADR-008 A2A boundary & ownership migration: conflict with `turul-a2a/crates/turul-a2a-patterns/src/manifest.rs::SkillCard::parse` (camelCase, `deny_unknown_fields`, required `id`, serde_yaml 0.9); D9 table: every `SkillCard` field → dropped / `metadata` key / `assets/` file; rejected alternatives (sidecar json, typed vendor fields, extension trait); what breaks today both ways; one migrated a2a example SKILL.md as fixture; contract file `a2a_boundary.rs` (migrated fixture strict-valid; no A2A symbol in public API); write the turul-a2a handoff (bottom) into ADR-008 §Handoff | `docs/adr/ADR-008-a2a-projection-boundary.md` | every `SkillCard` field has a row; handoff ≤15 lines — **drafted 2026-11-06; fixture at `house-a2a-migrated/greet/`**; stays Proposed (turul-a2a still has `SkillCard::parse`) | O | T1.6, D9 |
| T1.10 | ADR-009 discovery: `discover(scopes: &[Scope]) -> Discovered{skills, diagnostics}`, `Scope{kind: Project\|User, root}`; helper for conventional subpaths (`.agents/skills/`, `.claude/skills/`) the caller may bypass; skip `.git`, `node_modules`; named depth/dir caps; every candidate via `load` only; collisions: project beats user, within scope first in sorted path order, diagnostic per shadowed skill; output sorted by name; symlinks never followed outside root; trust gating out of scope. Contract file `discovery.rs` | `docs/adr/ADR-009-discovery.md` | every rule has a test name; collision table — **drafted 2026-11-06**; **Accepted 2026-11-06** | O | T1.5, D10 |
| T1.11 | ADR review pass: run `adr-review` over `docs/adr/`; fix drift | all ADRs | "No findings." — **done 2026-11-06** | O | T0.5, T1.1–T1.10 |

## P2 — Conformance fixtures (data + expected outcomes; no impl)
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T2.1 | Translate each collected skills-ref case into `tests/fixtures/<case>/SKILL.md` + `expected.json` (schema and `layout` per ADR-001 §2.2) | fixtures | count == ADR-001 table — **done 2026-11-06** | S | T0.7, T1.11 |
| T2.2 | House fixtures: `---` inside a quoted value; `version: 1.0`; int metadata value; CRLF; no trailing newline; `skill.md`; unknown key (`disable-model-invocation: true`); empty unknown key; unquoted colon in value; Unicode name; name >64; dir mismatch; migrated A2A skill (ADR-008) | fixtures | each in ADR-001 table — **done 2026-11-06** | S | T2.1 |
| T2.3 | Goldens: `expected.prompt.xml` for corpus cases **ported verbatim from `test_prompt.py` asserts**; hand-derived from ADR-005/006 only for house fixtures; `expected.render.md` per valid fixture | fixtures | `git diff --stat` shows only fixtures — **done 2026-11-06** | S | T2.2 |

## P3 — TDD per ADR (test red → implement → green; no API beyond the ADRs)
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T3.1 | `skill_name.rs`: each rule; distinct inputs → distinct names; compile-fail doctest for bypass | tests | red — **done 2026-11-06** | S | T1.11 |
| T3.2 | Implement `SkillName` | `src/name.rs` | T3.1 green — **done 2026-11-06** | S | T3.1 |
| T3.3 | `frontmatter.rs`: delimiters, `version: 1.0`, metadata coercion, `allowed-tools`, CRLF, colon fallback, unknown keys retained | tests | red — **done 2026-11-06** | S | T1.11 |
| T3.4 | Implement `SkillProperties`, `RawSkill`, `parse_skill_md`; YAML dep (D7) added to workspace deps | `src/parse.rs`, `Cargo.toml` | T3.3 green; one YAML crate in `cargo tree` — **done 2026-11-06** | S | T3.2, T3.3 |
| T3.5 | `conformance.rs` (table over `expected.json`) + `validate_and_load.rs` (all-errors order, codes, warn/skip matrix, unknown key strict+lenient, length/dir rules, no non-spec field on `SkillProperties`, never panics) | tests | red — **done 2026-11-06** | S | T2.2 |
| T3.6 | Implement `ValidationError`, `validate`, `load`, `Skill` | `src/validate.rs`, `src/load.rs`, `src/skill.rs` | T3.5 green — **done 2026-11-06** | S | T3.4, T3.5 |
| T3.7 | `render.rs`: goldens; `load(render(skill)) == skill`; extensions byte-stable after description edit; determinism; body untouched; `write_to` writes `SKILL.md` in `<name>/` | tests | red — **done 2026-11-06** | S | T2.3, T3.6 |
| T3.8 | Implement `Skill::render`, `Skill::write_to` | `src/render.rs` | T3.7 green — **done 2026-11-06** | S | T3.7 |
| T3.9 | `catalog_prompt.rs`: goldens 0/1/2; escaping; determinism; input order preserved; `activation` lists resources without reading them | tests | red — **done 2026-11-06** | S | T2.3, T3.6 |
| T3.10 | Implement `to_prompt`, `activation` | `src/prompt.rs` | T3.9 green — **done 2026-11-06** | S | T3.9 |
| T3.11 | `discovery.rs` on a tempdir tree: nested find; skips; caps; collision + diagnostic; determinism; sorted; symlink escape not followed; malformed → diagnostic, scan continues | tests | red — **done 2026-11-06** | S | T1.10, T3.6 |
| T3.12 | Implement `discover`, `Scope` | `src/discover.rs` | T3.11 green — **done 2026-11-06** | S | T3.11 |
| T3.13 | `a2a_boundary.rs`: migrated A2A fixture strict-valid; public API has no A2A symbol | tests | green — **done 2026-11-06** | S | T3.6 |
| T3.14 | `single_owner.rs`: grep `src/` of both crates + `examples/` (not `tests/`): no `SkillName(`/`SkillProperties {` outside owners; `parse_skill_md(` only in `validate.rs`/`load.rs`; `discover.rs` calls `load` only | tests | green; re-run in T4.2 and T4b.3 gates — **done 2026-11-06** | H | T3.12 |
| T3.15 | `lib.rs` surface = exactly the ADR-named symbols; rustdoc on each; `#![deny(missing_docs)]` | `src/lib.rs` | `cargo doc --no-deps` clean — **done 2026-11-06** | S | T3.12 |
| T3.16 | Root README draft as **`crates/turul-agent-skills/README.md`** (packaged with the crate), included via `#![doc = include_str!("../README.md")]`; root `README.md` is a symlink to it. User-focused: what Agent Skills are + spec link; install; quick start snippets — validate, load + read properties, create + `write_to`, modify an existing skill, `to_prompt`, `activation`, `discover`; CLI quick start; `examples/skill-llm` (offline + `RUN_OLLAMA_SMOKE=1`); divergences table last | README, `src/lib.rs` | `cargo test --doc -p turul-agent-skills` green — **done 2026-11-06** | S | T3.15 |
| T3.17 | Opus verification of P2–P3: full suite, every ADR vs code, fixtures vs pytest source, README accuracy; ADR revision logs updated; first commit allowed after this | ADRs | `adr-review` "No findings."; `cargo test --workspace` green — **verified 2026-11-06 (re-run in T5.4); not committed** | O | T3.13, T3.14, T3.16 |

## P4 — CLI
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T4.1 | `crates/turul-agent-skills-cli/tests/cli.rs`: `assert_cmd` over fixtures vs ADR-007 table | tests | red — **done 2026-11-06** | S | T1.8, T2.3 |
| T4.2 | Implement binary (`clap` derive); no format logic | `crates/turul-agent-skills-cli/src/main.rs` | T4.1 green; `cargo tree -e no-dev` = clap + lib; T3.14 green — **done 2026-11-06** | S | T3.6, T3.10, T4.1 |
| T4.3 | Differential vs skills-ref: `uvx --from "git+https://github.com/agentskills/agentskills#subdirectory=skills-ref" skills-ref <cmd> <fixture>` (confirm it reports version 0.1) (isolated install, scratchpad output) vs ours on every fixture | scratchpad | mismatches empty or all in ADR-001 — **done 2026-11-06** | S | T4.2 |

## P4b — Examples that USE skills (turul-llm; examples-only, see ADR-001 §deps)
Offline stub by default; live Ollama opt-in via `OLLAMA_BASE_URL`/`OLLAMA_MODEL` or `RUN_OLLAMA_SMOKE=1`, mirroring `turul-llm/examples/greet-ollama`. Never edit turul-llm.
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T4b.1 | Add `examples/skill-llm` to workspace members; add `turul-llm-core`/`turul-llm-ollama` `"0.1"` (sibling crates: major.minor only) to `[workspace.dependencies]`; write `examples/skill-llm/tests/offline.rs` running both bins with the stub | `Cargo.toml`, example | red — **done 2026-11-06** | S | T1.2, T3.17, D11 |
| T4b.2 | Bin `catalog`: `discover`/`load` fixture skills → `to_prompt` → `LlmClient` picks a name (schema enum) → `activation` → second call answers. Stub returns a fixed pick | `src/bin/catalog.rs` | `cargo run -p skill-llm --bin catalog` exits 0 offline — **done 2026-11-06** | S | T4b.1 |
| T4b.3 | Bin `author`: `LlmClient` drafts fields (`name`, `description`, body; JSON Schema) → `SkillName::parse` + `SkillProperties::new` + `Skill::new` → `write_to` (temp dir) → strict `validate` (errors printed, exit 1) → `load` → edit description → `write_to`. No text-to-`Skill` API | `src/bin/author.rs` | T4b.1 test green; output dir passes CLI `validate`; T0.6 + T3.14 green — **done 2026-11-06** | S | T4b.2, T4.2 |

## P5 — Release gate
| id | action | files | gate | tier | deps |
|----|--------|-------|------|------|------|
| T5.1 | Finalise README (CLI binary name, example commands); CLI crate README: brief usage + link | READMEs | `cargo test --doc --workspace` green; no AI attribution — **done 2026-11-06** | H | T4.3, T4b.3 |
| T5.2 | CHANGELOG `0.1.0` (adopter-facing); versions consistent | `CHANGELOG.md` | `cargo package -p turul-agent-skills -p turul-agent-skills-cli --no-verify --allow-dirty` clean — **done 2026-11-06** | H | T5.1 |
| T5.3 | Flip ADR-001..007, 009 → Accepted where no alternative path remains (ADR-008 stays Proposed until turul-a2a deletes `SkillCard::parse`); re-verify T4.3's divergence table | ADRs | each flip cites its test file — **done 2026-11-06** | O | T4.3, T4b.3, T5.1 |
| T5.4 | Final opus verification: fmt, clippy `-D warnings`, test, doc, package, `adr-review`; then commit — never commit-then-verify | — | all exit 0 — **verified 2026-11-06: fmt, clippy, test, doc, package, adr-review all exit 0 / no findings; commit left to the maintainer** | O | T5.2, T5.3 |

## Handoff — turul-a2a session (copied into ADR-008 by T1.9; do not execute here)
- Owner: turul-a2a session. Goal: replace `crates/turul-a2a-patterns/src/manifest.rs::SkillCard::parse` with `turul_agent_skills::load` + `SkillCard: TryFrom<&Skill>` reading namespaced `metadata` keys and `assets/*.json` files per ADR-008; `SkillCard::to_agent_skill` unchanged; migrate every example `SKILL.md` to spec frontmatter (`name` = dir name, drop `id`); `_` ids → `-` for `name` with the original id in `metadata.a2a-id`, and `SkillCard` projects `a2a-id` (else `name`) to `AgentSkill.id`, per ADR-008 §2.2; delete `serde_yaml` from the a2a workspace.
- Gates: `turul-skills validate` exits 0 on every a2a example skill; a2a tests and interop smoke green; `rg serde_yaml` empty; wire ids unchanged (interop clients and smoke tests untouched); a2a ADR revision log updated; patch bump + CHANGELOG by its owner.
- Stop: any A2A field without an ADR-008 row, or any `id` that is not a valid `SkillName` after `_` → `-` → report back, do not invent a mapping; any need to change this crate's API → report, do not fork.
