# ADR-007: CLI parity with skills-ref

- **Status:** Accepted (2026-11-06; contract tests `cli.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills-cli/tests/cli.rs` (workspace only, not published)
- **Depends on:** ADR-001 (corpus, divergences), ADR-004 (messages), ADR-006 (catalog bytes); D6, D8

## 1. Context

`skills-ref/src/skills_ref/cli.py` (click) defines three commands. Each argument is
`click.Path(exists=True, path_type=Path)`; `_is_skill_md_file(path)` is
`path.is_file() and path.name.lower() == "skill.md"`, and such a path is replaced by its parent.
Output goes through `click.echo` (adds `\n`). Click usage errors exit 2. Paths print through
`str(Path)`, which drops `.` components and trailing slashes.

Click 8.3.1 (the `uv.lock` version) was probed with a stand-in script declaring the same
arguments:

| Invocation | Exit | stderr |
|---|---|---|
| `validate nope` | 2 | `Usage: skills-ref validate [OPTIONS] SKILL_PATH` / `Try 'skills-ref validate --help' for help.` / blank / `Error: Invalid value for 'SKILL_PATH': Path 'nope' does not exist.` |
| `to-prompt` (no paths) | 2 | … / `Error: Missing argument 'SKILL_PATHS...'.` |
| `validate ./probe/my-skill/` | 0 | stdout `Valid skill: probe/my-skill` |

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Binary | `[[bin]] name = "turul-skills"` in `turul-agent-skills-cli` | `main` (clap derive) | `crates/turul-agent-skills-cli/src/main.rs` |
| Properties JSON | private `properties_json(&SkillProperties) -> String` | `read-properties` | same file |

The binary depends on `clap` and `turul-agent-skills` only (D8). Every message about a skill
comes from `ValidationError::message()` (ADR-004) and every path from the library's public `display_path`;
the CLI adds only the fixed frames below (`Valid skill: `, `Validation failed for …:`, `  - `,
`Error: `) and the JSON layout.

### 2.2 Commands

| Command | Argument handling | Library call |
|---|---|---|
| `validate <path>` | must exist, else usage error; a `skill.md` file (any case) → its parent | `validate(dir)` |
| `read-properties <path>` | same | `load(dir)` |
| `to-prompt <path>...` | one or more, each must exist; each `skill.md` file → its parent; each directory canonicalized before `load` (as `to_prompt`'s `Path.resolve()`, so error paths are absolute) | `load` each, then `to_prompt` |

### 2.3 Invocation table

`D` = a path argument as typed; `P` = `str(Path(D))` equivalent (library path text); `E` = the
first "skip" error of `LoadSkip` (ADR-004 §2.4); stdout and stderr end with `\n`.

| # | Invocation | Exit | stdout | stderr |
|---|---|---|---|---|
| 1 | `validate D`, no errors | 0 | `Valid skill: P` | — |
| 2 | `validate D/SKILL.md` | as `validate D` | | |
| 3 | `validate D`, errors e₁…eₙ | 1 | — | `Validation failed for P:` then `  - eᵢ` per error |
| 4 | `validate F` (a file not named `skill.md`) | 1 | — | `Validation failed for P:` / `  - Not a directory: P` |
| 5 | `read-properties D`, loads | 0 | properties JSON (§2.4) | — (diagnostics are not printed) |
| 6 | `read-properties D`, skipped, or loaded only through the colon repair | 1 | — | `Error: E`, the first skip error (`LoadSkip.errors[0]`); for the repair, `Error: Invalid YAML in frontmatter: {detail}` |
| 7 | `to-prompt D₁ … Dₙ`, all load without repair | 0 | `to_prompt` bytes (ADR-006 §2.2) | — |
| 8 | `to-prompt …`, any skipped or repaired | 1 | — | `Error: E` of the first skipped, in argument order |
| 9 | any command, path missing | 2 | — | click-shaped usage block printed by the CLI (`Usage: turul-skills <cmd> [OPTIONS] <ARG>`, `Try … --help`), last line `Error: Invalid value for 'SKILL_PATH': Path 'D' does not exist.` (`SKILL_PATHS` for `to-prompt`) |
| 10 | `to-prompt` with no paths | 2 | — | clap usage error |

Row 10 means the CLI never prints the empty catalog wrapper, exactly as `skills-ref` (its
`nargs=-1, required=True` rejects an empty list before `to_prompt([])` can run). D6's
"CLI emits empty wrapper" is unreachable in the reference and therefore not implemented.

Corpus mapping (`conformance` fixtures): rows 1/3 print the ADR-004 messages for the fixture's
`expected.json` codes; rows 5–8 use its `load` outcome.

### 2.4 Properties JSON

Python `json.dumps(props.to_dict(), indent=2)` byte for byte:

- keys in order `name`, `description`, then `license`, `compatibility`, `allowed-tools` when
  present, then `metadata` when non-empty;
- two-space indent; items separated by `,` plus newline; key and value separated by `": "`;
  `metadata` entries at four spaces; no trailing newline before `click.echo`'s;
- `ensure_ascii`: `"` → `\"`, `\` → `\\`, `\n` `\r` `\t` `\b` `\f` as escapes, other
  characters below U+0020 as `\u00XX`, every non-ASCII character as `\uXXXX` with lowercase hex,
  characters above U+FFFF as a surrogate pair; DEL printed raw.

Example (`p-read-metadata`):

```json
{
  "name": "my-skill",
  "description": "A test skill",
  "metadata": {
    "author": "Test Author",
    "version": "1.0"
  }
}
```

### 2.5 Rules and tests

| Rule | Test |
|---|---|
| Binary is `turul-skills` | `binary_name_is_turul_skills` |
| Rows 1–4 | `validate_valid_exit_0_stdout`, `validate_skill_md_path_uses_parent`, `validate_errors_exit_1_stderr_lines`, `validate_regular_file_not_a_directory` |
| Rows 5–6 | `read_properties_json_bytes`, `read_properties_skip_exit_1` |
| The CLI does not apply the colon fallback: `read-properties` and `to-prompt` exit 1 with `Error: Invalid YAML in frontmatter: …` on `house-colon-in-value` | `repaired_yaml_exit_1_like_skills_ref` |
| JSON escaping (non-ASCII, astral, controls) | `read_properties_json_ensure_ascii` |
| `read-properties` and `to-prompt` print the NFKC name: `v-nfkc` (`cafe` + U+0301) prints `caf\u00e9` and `café` | `printed_name_is_nfkc` |
| `read-properties` metadata keys are sorted: source order `version`, `author` prints `author`, `version` | `read_properties_metadata_sorted_when_source_unsorted` |
| Rows 7–8 | `to_prompt_bytes_match_library`, `to_prompt_skip_exit_1_absolute_path` |
| Rows 9–10 | `missing_path_exit_2`, `to_prompt_without_paths_exit_2` |
| Path text matches `pathlib` (`./x/` → `x`, `SKILL.md` → `.`) | `paths_print_like_pathlib` |
| Every corpus and house fixture vs the table | `cli_matches_table_on_every_fixture` |
| CLI depends on clap and the library only | `cargo tree -e no-dev -p turul-agent-skills-cli --depth 1` lists exactly those two |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Binary `skills-ref` | Collides with the reference on `PATH`; differential testing needs both |
| `serde_json` for `read-properties` | No `ensure_ascii` mode (needs a custom formatter); a third dependency beyond clap and the library (D8) |
| Print `load` diagnostics on stderr | `skills-ref` prints nothing for those cases; would break parity |
| Print the empty wrapper for zero paths | Unreachable in `skills-ref` |
| Reproduce click usage text | Program name and clap layout differ anyway; exit code and the final error line are the contract |

## 4. Consequences

- `--help`, `--version` and usage text are clap's (ADR-001 §2.3).
- The differential run against `skills-ref` 0.1 compares exit code, stdout and stderr per fixture; any mismatch not in
  ADR-001 §2.3 is a defect.
- The CLI treats a `load` result carrying the `yaml-repaired` diagnostic as the strict `yaml-invalid`
  error (the first message of `validate`), so `house-colon-in-value` exits 1 as in `skills-ref`.
  The library's `load` keeps the fallback for agent clients; no new library API.

## 5. Revision log

- 2026-11-06 — Initial. Binary `turul-skills`; invocation table; JSON byte rules; empty wrapper unreachable.
- 2026-11-06 — Task-plan ids replaced by the commands they named; `serde_json` rejection restated against D8.
- 2026-11-06 — Repaired-YAML behaviour of rows 5/7 stated and tested; divergence recorded in ADR-001.
- 2026-11-06 — Tests `printed_name_is_nfkc` and `read_properties_metadata_sorted_when_source_unsorted` added for the printed-name and metadata-order divergences.
- 2026-11-06 — Path text for `Valid skill:` and `Validation failed for` comes from the library's public `display_path`.
- 2026-11-06 — Default pending maintainer review: the CLI does not apply the colon fallback (rows 6, 8); `repaired_yaml_prints_exit_0` replaced by `repaired_yaml_exit_1_like_skills_ref`. ADR-001 divergence removed.
- 2026-11-06 — Row 6 `E` is the first skip error, ordered by the library (ADR-004); `skip_reason_is_the_error_that_caused_the_skip` added.
- 2026-11-06 — Accepted: `cli.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only). Defaults marked pending maintainer review stay marked; reversing one is a revision, not a new path.
- 2026-11-06 — Row 9 states the missing-path usage block is click-shaped and printed by the CLI; other usage errors come from clap.
- 2026-11-07 — `cli.rs` runs over the library's fixtures, so it is excluded from the published CLI package and runs under `cargo test --workspace` only (ADR-001).
