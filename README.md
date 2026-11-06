# Turul Agent Skills

Rust tooling for [Agent Skills](https://agentskills.io): the open `SKILL.md` format, stewarded
by the Linux Foundation's [Agentic AI Foundation](https://aaif.io/projects/agent-skills), for
packaging instructions, scripts and resources that an AI agent loads when a task calls for them.

Turul Agent Skills reads, validates, writes and catalogs skill directories, and ships a command
line that behaves like the specification's reference tool, `skills-ref`. It is the one place the
turul projects (such as [turul-a2a](https://github.com/aussierobots/turul-a2a)) create and change
`SKILL.md` files.

```text
greet-user/
├── SKILL.md        # YAML frontmatter (name, description, ...) + Markdown instructions
├── scripts/        # optional
├── references/     # optional
└── assets/         # optional
```

## Crates

| Crate | What it is |
|---|---|
| [`turul-agent-skills`](crates/turul-agent-skills) | Library: validate, load, create and modify skills, build the `<available_skills>` catalog, activate a skill, discover skills in a project. No async runtime, network or LLM dependency. |
| [`turul-agent-skills-cli`](crates/turul-agent-skills-cli) | The `turul-skills` binary: `validate`, `read-properties`, `to-prompt`, with `skills-ref` output and exit codes. |
| [`examples/skill-llm`](examples/skill-llm) | Skills driving an LLM through [turul-llm](https://github.com/aussierobots/turul-llm): pick and activate a skill, or draft and write one. Runs offline by default. |

## Quick start

```sh
cargo add turul-agent-skills
```

Validate a skill directory. An empty list means it is valid:

```rust
let errors = turul_agent_skills::validate(&dir);
assert!(errors.is_empty());
for error in &errors {
    println!("{}: {error}", error.code().as_str());
}
```

Create a skill and write it to `<parent>/<name>/SKILL.md`:

```rust
use turul_agent_skills::SkillBuilder;

let skill = SkillBuilder::new("summarize", "Summarize a document in three bullets.")
    .license("MIT")
    .body("Read the document, then answer in three bullets.\n")
    .build()
    .expect("a legal name and a non-empty description");

let file = skill.write_to(parent.path())?;
assert!(file.ends_with("summarize/SKILL.md"));
assert!(turul_agent_skills::validate(file.parent().unwrap()).is_empty());
```

Find the skills in a project and build the catalog a model sees at the start of a session:

```rust
use turul_agent_skills::{Scope, ScopeKind, discover};

let scopes = Scope::conventional(ScopeKind::Project, project.path());
let found = discover(&scopes);
assert_eq!(found.skills.len(), 1);
for diagnostic in &found.diagnostics {
    eprintln!("{}: {}", diagnostic.path.display(), diagnostic.error);
}
```

Loading, modifying, the catalog and activation are covered in the
[library guide](crates/turul-agent-skills/README.md), where every snippet is compiled and run as
a test.

## Command line

```sh
cargo install turul-agent-skills-cli
turul-skills validate path/to/skill          # exit 0 valid, 1 invalid
turul-skills read-properties path/to/skill   # frontmatter as JSON
turul-skills to-prompt path/to/a path/to/b   # <available_skills> block
```

## With an LLM

```sh
cargo run -p skill-llm --bin catalog     # the model picks a skill from the catalog, then uses it
cargo run -p skill-llm --bin author      # the model drafts a skill; it is validated, written, modified
cargo run -p skill-llm --bin smoke       # every public capability, one assertion per step
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin catalog   # live Ollama instead of the offline stub
RUN_OLLAMA_SMOKE=1 cargo run -p skill-llm --bin smoke
```

## Specification conformance

Turul Agent Skills follows the [Agent Skills specification](https://agentskills.io/specification)
and the [client implementation guide](https://agentskills.io/client-implementation/adding-skills-support),
and agrees with `skills-ref` 0.1: all 40 of its test cases run as this project's conformance
suite, and the CLI's output is compared with `skills-ref` on every fixture. Where the two differ,
the reason is recorded; see
[differences from skills-ref](crates/turul-agent-skills/README.md#spec-conformance-and-divergences).

## Project documentation

- [Architecture decision records](docs/adr): one per contract, each naming the tests that prove it.
- [CHANGELOG](CHANGELOG.md)
- [Contributor guidelines](AGENTS.md)

## Building

```sh
cargo build --workspace
cargo test --workspace      # no network needed
```

Rust 1.94 or later.

## Licence

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your
option.
