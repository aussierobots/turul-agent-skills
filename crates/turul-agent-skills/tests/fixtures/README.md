# Conformance fixtures

One directory per case. `conformance.rs` reads `expected.json`; `render.rs` and
`catalog_prompt.rs` read the goldens. The expected-outcome schema, the `layout` values and the
case table are in the spec-sources and conformance-corpus ADR.

Ids: `p-*`, `v-*`, `r-*` port the 40 `skills-ref` 0.1 test cases (parser, validator, prompt);
`house-*` are this crate's own.

## Layout

- `<id>/<dir>/SKILL.md` (or `skill.md`): the skill directory sits inside the case directory, so
  the directory-name rules (length, NFKC, Unicode, mismatch) are testable. `<dir>` is the
  directory name exactly as `skills-ref` creates it (`a` x 70, `café` U+00E9, `НАВЫК`, ...).
- `<id>/SKILL.md`: cases that parse a string, not a directory (`p-valid-frontmatter`,
  `p-missing-frontmatter`, `p-unclosed-frontmatter`, `p-invalid-yaml`, `p-non-mapping`).
- `<id>/expected.json` only: cases git cannot hold (`layout` set), or `r-empty`.
- `p-prefer-uppercase`: `my-skill/SKILL.md` plus `lowercase.md` (a case-insensitive checkout
  must hold both).
- `p-accept-lowercase` keeps the `skills-ref` body, the string `lowercase`, so it fails with
  `frontmatter-missing`: reaching that error proves `skill.md` was found.
- `v-nfkc`: directory `café` is U+00E9; `SKILL.md` names `cafe` + U+0301.

## Goldens

- `expected.render.md`: exact bytes `Skill::render` returns for the loaded skill. Present for every
  fixture whose `load` is `ok`, and for `house-unknown-key`, `house-empty-extension`,
  `house-colon-in-value` (extensions and repair).
- `expected.prompt.xml`: exact bytes `to_prompt` returns for the case's skills. In it `/T` stands
  for the canonical path of the case directory. `r-empty`'s file holds the `skills-ref` output for
  no skills; this crate returns `None` there (`prompt` is `null`).

## Bytes

`.gitattributes` sets `-text` so CRLF (`house-crlf`) and a missing final newline
(`house-no-trailing-newline`) survive checkout. Do not reformat fixture files.
