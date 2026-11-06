# ADR-009: Discovery

- **Status:** Accepted (2026-11-06; contract tests `discovery.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/discovery.rs`
- **Depends on:** ADR-002 (`SkillName` as collision key), ADR-004 (`load`, `find_skill_md`, `ErrorCode`); D5, D10

## 1. Context

The client guide ("Step 1: Discover skills") describes project and user scopes, the
`.agents/skills/` convention and client-specific directories (`.claude/skills/` noted as common),
subdirectories containing `SKILL.md`, skipping `.git/` and `node_modules/`, bounds ("max depth of
4-6 levels, max 2000 directories"), "project-level skills override user-level skills", a
consistent first-found or last-found rule within a scope, a warning per shadowed skill, and a
trust check for project skills. `skills-ref` has no discovery.

D10: build it; caller-supplied roots; bounded; project beats user; every candidate through
`load`; trust gating is the caller's; synchronous `std::fs`.

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Scope | `ScopeKind::{Project, User}`, `Scope { pub kind: ScopeKind, pub root: PathBuf }` | — | `src/discover.rs` |
| Conventional roots | `Scope::conventional(kind: ScopeKind, base: &Path) -> Vec<Scope>` → `[base/.agents/skills, base/.claude/skills]` | optional helper; callers may build `Scope`s directly | `src/discover.rs` |
| Result | `Discovered { skills: Vec<Skill>, diagnostics: Vec<Diagnostic> }`, `Diagnostic { path: PathBuf, error: ValidationError }`; `ScopeKind`, `Discovered` and `Diagnostic` are `#[non_exhaustive]`, `Scope` stays exhaustive (callers build it by literal) | `discover(scopes: &[Scope]) -> Discovered` | `src/discover.rs` |
| Bounds | `pub const MAX_SCAN_DEPTH: usize = 6`, `pub const MAX_SCAN_DIRS: usize = 2000` (per scope) | — | `src/discover.rs` |
| Skipped directories | `pub(crate) const SKIPPED_DIRS: [&str; 2] = [".git", "node_modules"]`, consumed by `discover` and by `activation` (ADR-006) | — | `src/discover.rs` |
| Collision key | `SkillName` (normalized) | — | ADR-002 |

### 2.2 Scan

Per scope, in the order given:

1. A missing root is skipped silently (absent conventional directories are normal). A root that
   cannot be read gives `scan-io`.
2. Walk directories below the root, depth-first, entries of each directory in byte order of
   their names. The root is depth 0 and is not a candidate. Directories at depth >
   `MAX_SCAN_DEPTH` are not entered.
3. Skip directories named `.git` or `node_modules`.
4. A directory where `find_skill_md` (ADR-004; D5: `SKILL.md`, else `skill.md`) finds a file is
   a candidate. It is passed to `load` and not descended into (its `scripts/`, `references/`,
   `assets/` are not skills).
5. Symbolic links to directories are followed only if their canonical target lies inside the
   canonical root; otherwise `symlink-escape` and skip. A candidate whose skill file resolves
   outside the root is skipped the same way. Each canonical directory is visited once (no
   cycles; overlapping scopes do not load a skill twice).
6. After `MAX_SCAN_DIRS` directories in one scope, stop that scope with `scan-limit-reached`.
7. `load` errors (`LoadSkip`) become one diagnostic per error; `Loaded.diagnostics` become
   diagnostics; the scan continues.

### 2.3 Collisions

Key: `SkillName`. Rank of a loaded skill = (kind: `Project` before `User`, scope position in the
input, scan order within the scope). The lowest rank wins. Each loser gives one `skill-shadowed`
diagnostic at the loser's path.

| Case | Winner | Diagnostic |
|---|---|---|
| Project and user skill share a name | project (whatever the input order) | `skill-shadowed` on the user skill |
| Two project scopes (e.g. `.agents/skills`, `.claude/skills`) | the scope earlier in the input | on the later one |
| Same scope, two paths | first in scan order (byte-sorted depth-first) | on the later one |
| Same canonical directory via two scopes | loaded once | none |

Output `skills` is sorted by `SkillName`. Diagnostics are in scan order, then shadowing in
output order. Same tree → same `Discovered`.

### 2.4 Diagnostic codes (shared `ErrorCode`, ADR-004)

| Code | Message |
|---|---|
| `skill-shadowed` | `Skill '{name}' at {path} is shadowed by {winner path}` |
| `scan-limit-reached` | `Stopped scanning {root} after {MAX_SCAN_DIRS} directories` |
| `symlink-escape` | `Not following {path}: it resolves outside {root}` |
| `scan-io` | `Cannot read {path}: {io error}` |

### 2.5 Rules and tests

| Rule | Test |
|---|---|
| Nested skills found; skill directories not descended into | `finds_nested_skills_and_stops_at_skill_dir` |
| `.git`, `node_modules` skipped | `skips_git_and_node_modules` |
| Depth cap | `depth_cap_enforced` |
| Directory cap with diagnostic | `dir_cap_enforced_with_diagnostic` |
| Project beats user, with `skill-shadowed` | `project_beats_user_with_shadow_diagnostic` |
| Same kind: earlier scope, then scan order | `within_kind_first_scope_then_sorted_path_wins` |
| Output sorted by name | `output_sorted_by_name` |
| Deterministic | `discover_is_deterministic` |
| Symlink outside root not followed; inside followed once | `symlink_escaping_root_not_followed`, `symlink_inside_root_followed_once` |
| Malformed skill → diagnostics, scan continues | `malformed_skill_is_diagnostic_scan_continues` |
| Missing root ignored | `missing_root_ignored` |
| Unreadable root gives `scan-io` | `unreadable_root_gives_scan_io` |
| Skill file symlinked outside the root skipped with `symlink-escape` | `skill_file_symlink_outside_root_skipped` |
| Same canonical directory via two scopes loaded once, no diagnostic | `overlapping_scopes_load_once` |
| Codes 30–33 have the exact §2.4 message text | `discovery_messages_match_text` |
| Lowercase `skill.md` discovered (D5) | `lowercase_skill_md_discovered` |
| `Scope::conventional` paths and order | `conventional_scopes_paths` |
| Candidates only through `load` | `discover_calls_load_only` (`single_owner.rs`) |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Built-in home/project path detection | Caller knows its client name, trust state and sandbox; D10 says caller-supplied roots |
| Trust gating inside `discover` | Policy belongs to the client; it chooses which scopes to pass |
| Last-found wins within a scope | Guide allows either; first-found matches project-before-user reasoning |
| Accept only exact `SKILL.md` (guide wording) | `load` and `validate` accept `skill.md` (D5); discovery must not disagree with them |
| `.gitignore` support | Optional in the guide; needs a matcher dependency; not needed for correctness |
| Follow every symlink | A project could point the scan at any directory on the machine |
| Async / parallel walk | D10: synchronous `std::fs`; no async in publish crates |

## 4. Consequences

- A skill installed by symlinking a directory from outside the root (`~/.claude/skills/x ->
  ~/dev/x`) is not discovered; the caller adds that directory as a scope instead.
- Shadowed skills are reported, never returned.

## 5. Revision log

- 2026-11-06 — Initial. Caller-supplied scopes; bounded depth-first scan; collision table; symlink containment.
- 2026-11-06 — Test names added for unreadable root, skill-file symlink escape, overlapping scopes and message text.
- 2026-11-06 — Accepted: `discovery.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only).
- 2026-11-06 — `SKIPPED_DIRS` is the single owner of the skipped directory names (ADR-006 consumes it). `find_skill_md` now lives in `validate.rs` (ADR-004). `ScopeKind`, `Discovered`, `Diagnostic` are `#[non_exhaustive]`.
