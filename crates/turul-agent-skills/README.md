# turul-agent-skills

Parse, validate, render and catalog [Agent Skills](https://agentskills.io).

An Agent Skill is a directory holding a `SKILL.md` file: YAML frontmatter that names and
describes a capability, then Markdown instructions for an LLM agent, optionally with bundled
scripts and references. An agent shows the model a catalog of names and descriptions, and loads
a skill's full instructions only when the task calls for it. See the
[specification](https://agentskills.io/specification).

This crate follows the Agent Skills specification and agrees with the `skills-ref` reference
implementation (version 0.1) on validation. It has no async runtime, no network and no LLM
dependency. Where it differs from `skills-ref`, the
[differences](#spec-conformance-and-divergences) are listed at the end.

## Install

```sh
cargo add turul-agent-skills                 # the library
cargo install turul-agent-skills-cli         # the `turul-skills` command
```

## Quick start

`use turul_agent_skills::prelude::*;` imports the types and functions below.

### Validate

`validate` checks a skill directory against the specification and returns every problem found.
An empty list means the skill is valid.

```rust
# let tmp = tempfile::tempdir()?;
# let dir = tmp.path().join("greet");
# std::fs::create_dir(&dir)?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\n---\nSay hello.\n")?;
let errors = turul_agent_skills::validate(&dir);
assert!(errors.is_empty());
for error in &errors {
    println!("{}: {error}", error.code().as_str());
}
# Ok::<(), std::io::Error>(())
```

### Load and read properties

`load` is lenient: a skill that is usable but imperfect loads with diagnostics, and only a
skill that cannot be used is skipped.

```rust
# let tmp = tempfile::tempdir()?;
# let dir = tmp.path().join("greet");
# std::fs::create_dir(&dir)?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\nlicense: MIT\n---\nSay hello.\n")?;
use turul_agent_skills::load;

match load(&dir) {
    Ok(loaded) => {
        let skill = &loaded.skill;
        assert_eq!(skill.properties.name.as_str(), "greet");
        assert_eq!(skill.properties.description, "Greet a user.");
        assert_eq!(skill.properties.license.as_deref(), Some("MIT"));
        assert_eq!(skill.body, "Say hello.\n");
        assert!(loaded.diagnostics.is_empty());
    }
    Err(skipped) => {
        for error in &skipped.errors {
            eprintln!("{error}");
        }
    }
}
# Ok::<(), std::io::Error>(())
```

A skill embedded in a binary (`include_str!`) loads with `load_str(dir_name, text)`: the same
rules, with `dir_name` standing in for the directory and no `location`.

### Create and write a skill

`SkillBuilder` parses the name and trims the description, so a `Skill` always has a legal identity
and a non-empty description. `build` returns `BuildError` otherwise.
`write_to` creates `<parent>/<name>/SKILL.md`.

```rust
use turul_agent_skills::SkillBuilder;

# let parent = tempfile::tempdir()?;
let skill = SkillBuilder::new("summarize", "Summarize a document in three bullets.")
    .license("MIT")
    .body("Read the document, then answer in three bullets.\n")
    .build()
    .expect("a legal name and a non-empty description");

let file = skill.write_to(parent.path())?;
assert!(file.ends_with("summarize/SKILL.md"));
assert!(turul_agent_skills::validate(file.parent().unwrap()).is_empty());
# Ok::<(), std::io::Error>(())
```

### Modify an existing skill

Load, change a field, write back. Frontmatter keys outside the specification and the body are
preserved.

```rust
# let parent = tempfile::tempdir()?;
# let dir = parent.path().join("greet");
# std::fs::create_dir(&dir)?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\n---\nSay hello.\n")?;
use turul_agent_skills::load;

let mut skill = load(&dir).expect("loads").skill;
skill.properties.description = "Greet a user warmly.".to_string();
skill.write_to(parent.path())?;

let reloaded = load(&dir).expect("loads").skill;
assert_eq!(reloaded.properties.description, "Greet a user warmly.");
assert_eq!(reloaded.body, "Say hello.\n");
# Ok::<(), std::io::Error>(())
```

### Catalog prompt

`to_prompt` renders the `<available_skills>` block a model sees at the start of a session. It
returns `None` when there are no skills, because an empty catalog is better omitted.

```rust
# let tmp = tempfile::tempdir()?;
# let dir = tmp.path().join("greet");
# std::fs::create_dir(&dir)?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\n---\nSay hello.\n")?;
use turul_agent_skills::{load, to_prompt};

let skills = vec![load(&dir).expect("loads").skill];
let catalog = to_prompt(&skills).expect("one skill");
assert!(catalog.starts_with("<available_skills>"));
assert!(catalog.contains("<name>\ngreet\n</name>"));
assert_eq!(to_prompt(&[]), None);
# Ok::<(), std::io::Error>(())
```

### Authoring advisories

`advise` reports the two measurable recommendations of the specification's progressive-disclosure
guidance: a `SKILL.md` over 500 lines, and file references nested deeper than one level. It is
separate from `validate` and `load`, which it never affects. Judging scope, gotchas or description
quality is review work, not the library's.

```rust
use turul_agent_skills::{ErrorCode, Skill, advise};

let long = Skill::builder("long", "A long skill.")
    .body("line\n".repeat(600))
    .build()
    .expect("a legal name and a non-empty description");

let advisories = advise(&long);
assert_eq!(advisories[0].code(), ErrorCode::SkillMdTooLong);
assert!(advisories[0].message().contains("500"));
```

### Activation

When the model asks for a skill, `activation` returns its instructions, its directory and the
names of the files bundled with it. Bundled files are listed, never read.

```rust
# let tmp = tempfile::tempdir()?;
# let dir = tmp.path().join("greet");
# std::fs::create_dir_all(dir.join("scripts"))?;
# std::fs::write(dir.join("scripts/hello.sh"), "echo hello\n")?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\n---\nSay hello.\n")?;
use turul_agent_skills::{activation, load};

let skill = load(&dir).expect("loads").skill;
let text = activation(&skill);
assert!(text.starts_with("<skill_content name=\"greet\">"));
assert!(text.contains("Say hello."));
assert!(text.contains("<file>scripts/hello.sh</file>"));
# Ok::<(), std::io::Error>(())
```

### Discover skills

`discover` scans the directories you give it, loads every skill it finds and resolves name
collisions: a project skill beats a user skill, and each skill that loses is reported as a
diagnostic. Which directories to scan, and whether a project is trusted enough to scan, is the
caller's decision.

```rust
# let project = tempfile::tempdir()?;
# let dir = project.path().join(".agents/skills/greet");
# std::fs::create_dir_all(&dir)?;
# std::fs::write(dir.join("SKILL.md"), "---\nname: greet\ndescription: Greet a user.\n---\nSay hello.\n")?;
use turul_agent_skills::{Scope, ScopeKind, discover};

let scopes = Scope::conventional(ScopeKind::Project, project.path());
let found = discover(&scopes);
assert_eq!(found.skills.len(), 1);
for diagnostic in &found.diagnostics {
    eprintln!("{}: {}", diagnostic.path.display(), diagnostic.error);
}
# Ok::<(), std::io::Error>(())
```

## Command line

`turul-agent-skills-cli` installs `turul-skills`, with the same `validate`, `read-properties`
and `to-prompt` commands, output and exit codes as `skills-ref`.

```console
$ turul-skills validate greet
Valid skill: greet

$ turul-skills read-properties greet
{
  "name": "greet",
  "description": "Greet a user.",
  "license": "MIT"
}

$ turul-skills validate bad
Validation failed for bad:
  - Skill name 'Bad' must be lowercase
  - Directory name 'bad' must match skill name 'Bad'
  - Missing required field in frontmatter: description
$ echo $?
1
```

`to-prompt greet` prints the same `<available_skills>` block as the library's `to_prompt`.

## Examples with an LLM

[`examples/skill-llm`](https://github.com/aussierobots/turul-agent-skills/tree/main/examples/skill-llm) uses [turul-llm](https://github.com/aussierobots/turul-llm) to let a model
pick a skill from the catalog and use it (`catalog`), and to draft, validate, write and modify
a skill (`author`); `smoke` asserts every public capability step by step. They run offline against a stub model by default:

```sh
cargo run -p skill-llm --bin catalog
cargo run -p skill-llm --bin author
cargo run -p skill-llm --bin smoke
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin catalog   # a live local Ollama model
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin smoke
```

## Spec conformance and divergences

The specification is followed; where `skills-ref` 0.1 and the specification differ, or
`skills-ref` has a defect, the choice is recorded in
[ADR-001](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-001-spec-sources-and-conformance-corpus.md)
and pinned by a test. All 40 `skills-ref` test cases are ported as fixtures and run as a
conformance suite. Beyond the topics below, the CLI matches `skills-ref` output and exit codes;
usage and `--help` text come from clap.

| Topic | skills-ref 0.1 | This crate | Decided in |
|---|---|---|---|
| Opening delimiter | Anything starting with `---` passes | Line 1 is exactly `---` | [ADR-003](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-003-frontmatter-parsing.md) |
| Closing delimiter | A `---` inside a value ends the block | The first later line that is exactly `---` | ADR-003 |
| Optional field types | `license`, `allowed-tools`, `metadata` not type-checked | Strings, string, and a string-to-string mapping | [ADR-004](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-004-strict-validation-and-lenient-loading.md) |
| Empty `compatibility` | Accepted | Rejected (the specification requires 1-500 characters) | ADR-004 |
| Unreadable or non-UTF-8 file | Uncaught exception | A `read-failed` error | ADR-004 |
| Unquoted `: ` in a value | An error | Repaired by the library's `load`; `validate` and the CLI report it, as `skills-ref` does | ADR-003, [ADR-007](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-007-cli-parity.md) |
| Invalid names when loading | Printed unchecked | Skipped with the rule's message (the client guide relaxes this) | [ADR-002](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-002-skill-name-identity.md), ADR-004 |
| Printed name | As written | NFKC-normalized | ADR-002 |
| `read-properties` metadata order | Source order | Sorted by key | ADR-003, [ADR-005](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-005-rendering.md) |
| Empty catalog | An empty wrapper element | `None` (the CLI still exits 2 without paths) | [ADR-006](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-006-catalog-prompt.md), ADR-007 |
| Discovery | None | `discover` (project before user, bounded, symlink-contained) | [ADR-009](https://github.com/aussierobots/turul-agent-skills/blob/main/docs/adr/ADR-009-discovery.md) |

Licensed under MIT OR Apache-2.0.
