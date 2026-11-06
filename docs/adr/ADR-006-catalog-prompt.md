# ADR-006: Catalog prompt and activation

- **Status:** Accepted (2026-11-06; contract tests `catalog_prompt.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/catalog_prompt.rs`
- **Depends on:** ADR-001 (corpus `r-*`), ADR-004 (`Skill`, `location`); D6

## 1. Context

`skills-ref` `prompt.py::to_prompt` builds the catalog line by line and joins with `\n`:
`<available_skills>`, then per skill `<skill>`, `<name>`, `html.escape(name)`, `</name>`,
`<description>`, `html.escape(description)`, `</description>`, `<location>`, `str(path)`,
`</location>`, `</skill>`, then `</available_skills>`. The location is not escaped. An empty
list returns `<available_skills>\n</available_skills>`. Python `html.escape(s, quote=True)` maps
`&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `'` → `&#x27;`.

The client guide: "If no skills are discovered, omit the catalog and behavioral instructions
entirely. Don't show an empty `<available_skills/>` block" ("When no skills are available").
"Structured wrapping" shows a `<skill_content name="…">` wrapper with the body, the skill
directory and a `<skill_resources>` list; "Listing bundled resources" says to list files but
"not eagerly read them" and to consider capping large listings.

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Catalog | — | `to_prompt(&[Skill]) -> Option<String>` | `src/prompt.rs` |
| Activation text | — | `activation(&Skill) -> String` | `src/prompt.rs` |
| Escaping | private `escape_html(&str) -> String` (the five mappings above, `&` first) | used by both | `src/prompt.rs` |
| Skipped directories | consumes `SKIPPED_DIRS` and `MAX_SCAN_DEPTH` from ADR-009; defines neither | used by `activation` | `src/prompt.rs` |
| Resource cap | `pub const MAX_RESOURCE_FILES: usize = 100` | used by `activation` | `src/prompt.rs` |

### 2.2 Catalog bytes

- Empty input → `None` (D6, client guide). This is the one library divergence (ADR-001 §2.3).
- Otherwise `skills-ref` layout exactly, skills in input order, no deduplication, no trailing
  newline. `name` is the normalized `SkillName`; `description` is the stored (trimmed) value;
  both are escaped.
- `<location>` holds `location().to_string_lossy()` **unescaped** (parity). The model uses it as
  a file-read argument, and an escaped path would name a different file. A skill without a
  location (built with `SkillBuilder`) gets no `<location>` lines; the client guide makes the field
  optional.

Golden bytes (`\n` shown as line breaks; no final newline):

| Input | Output |
|---|---|
| `[]` | `None` |
| one skill `my-skill`, `A test skill`, location `/T/my-skill/SKILL.md` | `<available_skills>` `<skill>` `<name>` `my-skill` `</name>` `<description>` `A test skill` `</description>` `<location>` `/T/my-skill/SKILL.md` `</location>` `</skill>` `</available_skills>` — 13 lines |
| `skill-a` (`First skill`), `skill-b` (`Second skill`) at `/T/skill-a/SKILL.md`, `/T/skill-b/SKILL.md` | `<available_skills>`, the 11-line `<skill>` block for `skill-a`, the 11-line block for `skill-b`, `</available_skills>` — 24 lines |
| description `Use <foo> & <bar> tags` | description line `Use &lt;foo&gt; &amp; &lt;bar&gt; tags` |

### 2.3 Activation bytes

```
<skill_content name="{escape_html(name)}">
{body.trim()}

Skill directory: {dir}
Relative paths in this skill are relative to the skill directory.

<skill_resources>
  <file>{relative path}</file>
</skill_resources>
</skill_content>
```

- `{dir}` = parent of `location`; it and every `<file>` path are unescaped (same reason as `<location>`). Without a location
  the two directory lines, the blank line before them and the resource block are omitted.
- Resources: every regular file under `{dir}`, recursively, except the skill file itself;
  relative paths with `/`; sorted by byte order; `.git` and `node_modules` skipped; symlinks
  listed as files, never followed; depth limited by ADR-009 `MAX_SCAN_DEPTH`. Names are listed,
  never opened. Unreadable directories are skipped silently.
- More than `MAX_RESOURCE_FILES` → the first 100 `<file>` lines, then the line
  `  (listing truncated at 100 files)`.
- No resources → the `<skill_resources>` block and the blank line before it are omitted.
- The text ends with `</skill_content>` and no newline.

### 2.4 Rules and tests

| Rule | Test |
|---|---|
| Empty input → `None` (D6) | `to_prompt_empty_is_none` |
| One and two skills: exact bytes of §2.2 | `to_prompt_one_skill_exact_bytes`, `to_prompt_two_skills_exact_bytes` |
| Name and description escaped as `html.escape(quote=True)` | `to_prompt_escapes_like_html_escape` |
| Location not escaped | `to_prompt_location_not_escaped` |
| Input order kept, duplicates kept | `to_prompt_preserves_input_order` |
| Same input → same bytes | `to_prompt_is_deterministic` |
| Goldens `expected.prompt.xml` of `r-single`, `r-multiple`, `r-escaping` match (their `skills-ref` assertions are `expected.json` `prompt`, checked by `conformance.rs`); `/T` in a golden is the canonical case directory; `r-empty`'s golden is the `skills-ref` empty-wrapper bytes and is not compared (the library returns `None`) | `to_prompt_matches_corpus_goldens` |
| Activation: trimmed body, directory lines, exact bytes | `activation_wraps_trimmed_body_with_directory` |
| Resources sorted, never opened (a `chmod 000` file is still listed) | `activation_lists_resources_sorted_without_reading` |
| No resources → no block | `activation_omits_resources_block_when_none` |
| Cap at `MAX_RESOURCE_FILES` with truncation line | `activation_caps_resource_listing` |
| Name attribute is the skill name (a `SkillName` holds no escapable character, so escaping cannot be observed) | `activation_name_attribute_is_the_skill_name` |
| Skill without a location: no `<location>` lines | `to_prompt_omits_location_without_one` |
| Activation without a location: no directory lines, no resource block | `activation_without_location_omits_directory_and_resources` |
| Symlinks listed, never followed; depth capped at `MAX_SCAN_DEPTH`; `.git`, `node_modules` skipped | `activation_does_not_follow_symlinks_and_caps_depth` |

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Return the empty wrapper (`skills-ref`) | Client guide says omit it; `None` makes "nothing to show" explicit |
| Escape `<location>` | Breaks the path as a file-read argument; `skills-ref` does not escape |
| Indented XML like the client guide sample | `skills-ref` bytes are the corpus; the guide says any format works |
| Read resource files in `activation` | Client guide: list, do not load |
| No cap | Unbounded output for large skill directories |

## 4. Consequences

- `skills-ref` `to_prompt` reads files; this crate's `to_prompt` takes already loaded skills.
  The CLI loads first (ADR-007).
- Paths that are not UTF-8 print lossily.

## 5. Revision log

- 2026-11-06 — Initial. `skills-ref` byte layout; location unescaped (parity); `None` on empty; activation wrapper with capped resource list.
- 2026-11-06 — `<file>` resource paths stated as unescaped, like `<location>`.
- 2026-11-06 — Test names added for location-less catalog and activation, symlinks and depth cap.
- 2026-11-06 — `to_prompt_matches_corpus_goldens` checks goldens only; the corpus `skills-ref` assertions are `expected.json` `prompt` (ADR-001 §2.2).
- 2026-11-06 — Golden placeholder `/T` and the `r-empty` reference-only golden stated.
- 2026-11-06 — `activation_escapes_name_attribute` replaced by `activation_name_attribute_is_the_skill_name`: `SkillName` admits only letters, digits and hyphens, so the escape is unobservable.
- 2026-11-06 — Accepted: `catalog_prompt.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only).
- 2026-11-06 — Location-less skills are built with `SkillBuilder` (ADR-004); `Skill::new` is crate-private.
- 2026-11-06 — The `.git`/`node_modules` skip list is the crate-private `SKIPPED_DIRS` owned by `discover.rs` (ADR-009); `prompt.rs` consumes it.
