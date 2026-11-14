# ADR-011: MCP projection boundary and bundled files

- **Status:** Proposed (stays Proposed until turul-mcp-framework serves skills through this crate's API with no parsing, validation or directory walk of its own)
- **Date:** 2026-11-14
- **Implements:** `crates/turul-agent-skills/tests/bundled_files.rs`, `crates/turul-agent-skills/tests/single_owner.rs` (workspace only, not published), `crates/turul-agent-skills/tests/dependency_direction.rs` (workspace only, not published)
- **Depends on:** ADR-001 (§deps), ADR-003, ADR-004, ADR-006, ADR-008, ADR-009

## 1. Context

turul-mcp-framework will serve Agent Skills over MCP, implementing the MCP skills extension
(`io.modelcontextprotocol/skills`, beta). It needs to load, validate and discover skills, and to
publish each skill's bundled files with a sha256 manifest over them. The manifest is only
meaningful if every party enumerates the same files, so the enumeration needs one owner.

Before this ADR the walk was the private `list_resources` in `src/prompt.rs`, capped at
`MAX_RESOURCE_FILES` and at `MAX_SCAN_DEPTH` levels, and used only by `activation`.

The specification does not mention symbolic links or hidden files. The client guide says an
activation listing "can also enumerate supporting files (scripts, references, assets) in the
skill directory" and, for large directories, to "consider capping the listing and noting that it
may be incomplete". Beyond "the files in the directory", §2.2 applies three rules the guide does
not state, each listed there with its source: `.git` is not entered (version-control
metadata, not skill content; maintainer decision D19); sockets, FIFOs and devices are not files
and are not listed; a name that cannot be represented as UTF-8 fails the walk rather than being
listed mangled.

## 2. Decision

### 2.1 Owners

| Artifact | Owner | Entrypoint |
|---|---|---|
| `SKILL.md` parse, validation, load, discovery | this crate | `load`, `load_str`, `validate`, `discover` (ADR-004, ADR-009) |
| A skill's bundled files | this crate | `bundled_files(&Skill) -> io::Result<Vec<String>>`, `src/bundle.rs` |
| Activation listing | this crate | `activation(&Skill)` lists the first `MAX_RESOURCE_FILES` of `bundled_files` (ADR-006) |
| MCP resources, URIs, the extension's wire format, the sha256 manifest | turul-mcp-framework | computed over `bundled_files`, reading the files it lists |

This crate's public API names no MCP type, URI scheme or concept, and no publish crate depends
on any `turul-mcp*` crate.

### 2.2 `bundled_files`

Every file under the skill directory (the parent of `Skill::location`) except the skill file
itself:

- relative paths, `/`-joined, sorted by byte order: the same directory gives the same list;
- complete: no count cap and no depth cap (recursion is finite because links are never followed);
- a symbolic link is listed under its own name and never followed, as `activation` always did;
  a caller that opens one resolves it and decides whether a target outside the directory is
  acceptable;
- `.git` is not entered (version-control metadata); every other directory is content, `node_modules`
  included, because the spec lets a skill directory hold "any files and directories". The client
  guide's `.git`/`node_modules` skip is a discovery rule ("directories that won't contain skills")
  and stays in `SKIPPED_DIRS`, used by discovery and `advise` (ADR-009, ADR-010); hidden names are included;
- sockets, FIFOs and devices are skipped;
- a skill without a location (built, or from `load_str`) has none: `Ok(vec![])`;
- an unreadable directory or entry, or a non-UTF-8 name, is an error; a partial list is never
  returned.

### 2.3 What the framework may rely on, and must not do

May rely on: `load`/`load_str` diagnostics and `LoadSkip` (ADR-004); `validate` (strict, every
error); `discover` scopes and precedence (ADR-009); `SkillName` as the identity (ADR-002);
`bundled_files` as the complete, deterministic file list; `render`/`write_to` round-trip
(ADR-005); `RawValue` as the raw frontmatter value, with its `typed` flag on plain scalars
(ADR-003).

Must not: parse or re-validate `SKILL.md`, walk a skill directory, or build a name other than
through `SkillName::parse`. A gap is a request to this crate, not a local implementation.

### 2.4 Not provided

- A typed YAML 1.2 core-schema resolver for `RawValue`, and a typed flag on map keys. The spec's
  fields are strings, and the framework's first cut needs strings only. Nothing breaks without
  them; a consumer need reopens this.

### 2.5 Rules and tests

| Rule | Test |
|---|---|
| Every file except the skill file, byte order | `lists_every_file_except_skill_md_in_byte_order` |
| Deterministic | `same_directory_gives_same_list` |
| No count cap | `has_no_cap` |
| Hidden and `node_modules` included; `.git` skipped at any depth | `includes_hidden_files_and_node_modules_and_skips_git` |
| No depth cap | `lists_every_depth` |
| Symlinks listed, never followed | `lists_symlinks_without_following_them` |
| No location → empty | `skill_without_location_has_none` |
| `skill.md` is the skill file when it is the one loaded | `lowercase_skill_md_is_the_skill_file` |
| Non-UTF-8 name → `InvalidData`, no partial list | `non_utf8_name_is_an_error_not_a_partial_list` |
| `activation` lists the first `MAX_RESOURCE_FILES` of the same walk | `activation_lists_the_first_files_of_the_same_walk` |
| Only `bundle.rs` and `discover.rs` read directories | `bundled_file_walk_lives_in_bundle_only` (`single_owner.rs`) |
| No publish crate depends on MCP crates | `publish_crates_have_no_forbidden_dependencies` (`dependency_direction.rs`) |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Skip symlinks so every listed file lies inside the directory | An exception the specification does not make, and it would hide linked files from activation; containment is the reader's decision |
| Keep the depth cap | It drops files silently, against the client guide's "noting that it may be incomplete"; without following links the walk is finite |
| Reuse discovery's `SKIPPED_DIRS` (also skip `node_modules`) | That rule is about where skills are not found; inside a skill the spec allows any files, so skipping `node_modules` would be an exception the spec does not make |
| Let the framework walk the directory | Two walks give two manifests; one owner per derived artifact |
| `Vec<PathBuf>` | Paths go into URIs and a manifest; `/`-joined UTF-8 strings are the portable form, and a non-UTF-8 name is reported rather than mangled |

## 4. Consequences

- `activation` lists files at every depth; before, files deeper than `MAX_SCAN_DEPTH` were
  dropped without a note. When the walk fails, `activation` lists no files; before, unreadable
  directories were skipped and the rest listed. `bundled_files` reports the error.
- `MAX_SCAN_DEPTH` now bounds discovery only.

## 5. Handoff — turul-mcp-framework (mcp-skills session; not executed here)

- Owner: mcp-skills. Goal: build the MCP skills extension on `load`/`discover`/`validate` and
  `bundled_files`; the manifest is sha256 over exactly `bundled_files`.
- Gates: no SKILL.md parsing, validation or directory walk in the framework; its skills pass
  `turul-skills validate`.
- Stop: any need for a different file policy or a new API → request it here, do not fork.

## 6. Revision log

- 2026-11-14 — Initial. `bundled_files` public, complete and uncapped; symlinks listed, never followed (unchanged from `activation`); depth cap removed for a skill's own files; MCP owner split; typed resolver not provided.
- 2026-11-14 — Maintainer review (D19): `.git` is excluded as version-control metadata; `node_modules` is listed as skill content (spec: "any files and directories"); device nodes are not listed; a non-UTF-8 name fails the walk.
