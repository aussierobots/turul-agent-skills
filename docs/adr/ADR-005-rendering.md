# ADR-005: Rendering

- **Status:** Accepted (2026-11-06; contract tests `render.rs` green)
- **Date:** 2026-11-06
- **Implements:** `crates/turul-agent-skills/tests/render.rs`
- **Depends on:** ADR-003 (YAML subset, verbatim body), ADR-004 (`Skill`, `load`); D4, D5

## 1. Context

`skills-ref` reads but never writes. Authoring tools and the `author` example need to create a
skill, change a field and write it back without disturbing anything else. Writing is only safe
if the output reloads to the same value, whatever YAML parser another client uses.

## 2. Decision

### 2.1 Owners

| Artifact | Owning symbol | Entrypoint | File |
|---|---|---|---|
| Canonical `SKILL.md` text | `Skill` | `Skill::render(&self) -> String` | `src/render.rs` |
| Skill directory on disk | `Skill` | `Skill::write_to(&self, parent: &Path) -> io::Result<PathBuf>` | `src/render.rs` |

`write_to` creates `parent/<name>/` (`create_dir_all`), writes `SKILL.md` (D5: never
`skill.md`), and returns the file path. It writes nothing else; resources are the caller's.

### 2.2 Layout

```
---
name: <name>
description: <description>
license: <license>                 (if Some)
compatibility: <compatibility>     (if Some)
metadata:                          (if non-empty)
  <key>: <value>                   (keys in BTreeMap order = byte order)
allowed-tools: <allowed-tools>     (if Some)
<extension key>: <value>           (each extension, insertion order)
---
<body, verbatim>
```

- Every line ends with `\n`. The output is the frontmatter, then `---\n`, then `body` unchanged
  (so an empty body ends the file with `---\n`).
- Nesting is two spaces per level. `RawValue::Map` entries go on the following lines.
  `RawValue::Seq` items are `- <scalar>`; an item that is a collection is `-` alone, with its
  content on the following lines, indented one more level.
- `Seq` and `Map` are never empty: `parse_skill_md` cannot produce an empty one (flow `[]`/`{}`
  is rejected) and `Skill.extensions` is read-only outside the crate (ADR-004).
- A `RawValue::Str` with `typed: true` (ADR-003 §2.3) is written plain, as its text, with no
  quoting; an empty one is written as nothing after `key:` or `-`. Its text came from a plain
  scalar that is null, bool, int or float, so it is valid plain YAML and reloads with the same
  text and `typed: true`. `disable-model-invocation: true` and `foo:` render byte-identical.

### 2.3 Scalar quoting

Applies to every value except a `typed: true` extension scalar (§2.2), and to every key that is
not a spec field name. The first matching row wins.

| # | Condition | Form |
|---|---|---|
| 1 | Contains a C0 control (including `\t`, `\n`, `\r`), DEL, a C1 control (U+0080–U+009F), U+FEFF, U+2028 or U+2029 | double-quoted |
| 2 | Empty | `''` |
| 3 | Leading or trailing whitespace | single-quoted |
| 4 | First character is one of `` - ? : , [ ] { } # & * ! \| > ' " % @ ` `` or the text starts with `...` | single-quoted |
| 5 | Contains `": "` or `" #"`, or ends with `:` | single-quoted |
| 6 | Matches a non-string scalar of YAML 1.2 core or YAML 1.1: `~`, `null`/`Null`/`NULL`, `true`/`false` (3 casings), `y`/`n`/`yes`/`no`/`on`/`off` (3 casings each), integer `[-+]?[0-9]+`, `[-+]?0o[0-7_]+`, `[-+]?0x[0-9a-fA-F_]+`, float `[-+]?(\.[0-9]+\|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`, `[-+]?\.(inf\|Inf\|INF)`, `\.(nan\|NaN\|NAN)`, and the YAML 1.1 forms: `0b[01_]+`, `_` in integers and floats (`1_000`), base-60 (`1:30`, `190:20:30.15`), timestamps (`2024-01-01`, `2024-1-1 10:00:00.5 +1`) | single-quoted |
| 7 | Otherwise, including non-ASCII printable text | plain |

Single-quoted: `'` doubled, nothing else escaped. Double-quoted escapes: `\\`, `\"`, `\t`, `\n`,
`\r`; other characters from row 1 as `\xNN` (≤ U+00FF) or `\uNNNN`, hex digits lowercase. Non-ASCII printable
characters are written as UTF-8, never escaped.

Row 6 exists for other clients: this crate reads every scalar as a string, but a typed YAML
parser would read `version: 1.0` as a float (the spec's own example quotes it: `version: "1.0"`).
It applies to spec fields, `metadata` (string values by the spec) and extension scalars with
`typed: false` (quoted in the source, so a string to every client).

### 2.4 Rules and tests

| Rule | Test |
|---|---|
| Field order: spec fields, then extensions in insertion order | `render_field_order_is_spec_order_then_extensions` |
| Absent optional fields and empty metadata are omitted | `render_omits_absent_optional_fields_and_empty_metadata` |
| Metadata keys in byte order | `render_metadata_keys_sorted` |
| Every row of §2.3, one case each, plus `'` and `"` inside values | `render_quoting_table` |
| `typed: true` extension scalars render plain: `house-unknown-key` (`disable-model-invocation: true`) and `house-empty-extension` (`foo:`) render byte-identical; `foo: 'true'` stays quoted | `render_preserves_plain_extension_scalars` |
| Outside the crate, extensions cannot be set: `skill.extensions.push(..)` does not compile | `compile_fail` doctest on `Skill::extensions` |
| Body bytes unchanged (CRLF, no trailing newline, leading blank lines) | `render_body_verbatim` |
| Same `Skill` → same bytes | `render_is_deterministic` |
| `load(render(skill)) == skill` on every valid fixture, extensions included | `load_render_round_trip_on_every_valid_fixture` |
| `render(load(render(s))) == render(s)` | `render_is_idempotent` |
| Changing `description` changes only the description line | `extensions_byte_stable_after_description_edit` |
| `write_to` creates `<name>/SKILL.md` and returns its path | `write_to_creates_name_dir_and_uppercase_file` |
| `render` equals `expected.render.md` per valid fixture | `render_matches_golden` |

`load(render(skill))` means: write `render(skill)` to `<tmp>/<name>/SKILL.md`, then `load` that
directory. Equality is `Skill`'s `PartialEq` (properties, extensions, body; ADR-004).

## 3. Alternatives considered

| Alternative | Rejected because |
|---|---|
| Keep the source frontmatter bytes and patch edited lines | Needs a span-preserving YAML editor; canonical output is simpler and deterministic |
| Metadata in source order | Output would depend on edit history; sorted keys give one canonical form |
| Always quote | Noisy diffs for ordinary values; plain text is valid when §2.3 allows |
| Double quotes as the default quote | `"` inside JSON-like values would need escaping; single quotes need none |
| Flow style for extensions | This crate's parser rejects flow style (ADR-003) |
| Escape non-ASCII | Unreadable non-English skills; YAML is UTF-8 |
| Apply row 6 to every extension scalar | `disable-model-invocation: true` becomes `'true'`, a string to the client that owns the key; D4 says extensions are kept verbatim |
| Keep every scalar's source style | A plain or quoted string that is not row-6 text means the same to every parser; recording more than `typed` adds state without changing meaning |
| Public `extensions`, render rejects spec-named or duplicate keys | Adds a render error path for values only callers could build; read-only `extensions` makes them unbuildable |

## 4. Consequences

- Rendering a skill loaded from a hand-written file normalizes its frontmatter (order, quoting,
  comments dropped). The body is never touched.
- A file written by `write_to` passes `validate` whenever the `Skill` satisfies ADR-004. `SkillBuilder::build` guarantees a legal name and a trimmed, non-empty description; the length limits remain `validate` rules (`padded_description_is_trimmed_and_round_trips`).

## 5. Revision log

- 2026-11-06 — Initial. Layout, quoting table, round-trip and idempotence rules.
- 2026-11-06 — Metadata-order alternative restated without a task-plan id.
- 2026-11-06 — `typed: true` extension scalars render plain (row 6 no longer turns `true` into `'true'` for the owning client); empty-collection rule removed (unreachable with read-only `extensions`); tests `render_preserves_plain_extension_scalars` and the `extensions` compile-fail doctest added.
- 2026-11-06 — Double-quoted hex escapes stated lowercase (implementation and `render_quoting_table`).
- 2026-11-06 — Row 6 extended with the YAML 1.1 binary, underscore, base-60 and timestamp forms (`render_quoting_table`). Keys longer than 1024 characters render in explicit `? key` / `:` form so the output reloads (`render_round_trips_keys_longer_than_a_simple_key`).
- 2026-11-06 — Accepted: `render.rs` green under `cargo test --workspace`; no alternative path remains (`single_owner.rs` green: one parser, `discover` through `load` only).
- 2026-11-06 — Row 6: signed hex and octal (`-0x1f`, `+0o17`) and `_` in octal quote, matching YAML 1.1 `[-+]?0x…` (`render_quoting_table`).
- 2026-11-06 — "`write_to` output passes `validate`" now holds for the description: `SkillBuilder::build` rejects an empty one and trims padding (ADR-004).
