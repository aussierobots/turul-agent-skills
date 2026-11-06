# turul-agent-skills-cli

`skills-ref` compatible command line for [Agent Skills](https://agentskills.io/specification),
built on [`turul-agent-skills`](https://crates.io/crates/turul-agent-skills).

```sh
cargo install turul-agent-skills-cli
turul-skills validate path/to/skill          # exit 0 valid, 1 invalid
turul-skills read-properties path/to/skill   # frontmatter as JSON
turul-skills to-prompt path/to/skill-a path/to/skill-b   # <available_skills> block
```

Output and exit codes match `skills-ref` 0.1; usage and `--help` text come from clap. See the
[library README](https://github.com/aussierobots/turul-agent-skills/blob/main/crates/turul-agent-skills/README.md)
for the quick start and the list of differences from `skills-ref`.
