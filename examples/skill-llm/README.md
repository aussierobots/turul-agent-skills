# skill-llm

Runnable examples of Agent Skills driving an LLM through
[turul-llm](https://github.com/aussierobots/turul-llm). Not published; examples only.

```sh
cargo run -p skill-llm --bin catalog   # the model picks a skill from the catalog, then uses it
cargo run -p skill-llm --bin author    # the model drafts a skill; it is validated, written, modified
cargo run -p skill-llm --bin smoke     # every public library capability, asserted step by step;
                                       # a model also drafts, revises and reviews a skill
```

All three run against an offline stub model by default, so `cargo test` needs no network.
For a live model:

| Variable | Effect |
|---|---|
| `RUN_OLLAMA_SMOKE=1` | Use Ollama at `http://localhost:11434` |
| `OLLAMA_BASE_URL` | Use Ollama at this URL (implies live mode) |
| `OLLAMA_MODEL` | Model name, default `llama3.1` |
| `OLLAMA_NUM_CTX` | Context window in tokens; set `16384` for thinking models such as `gemma4` (Ollama may default to 4096) |

`skills/` holds the example skills; `skills/skill-review/` reviews another skill against the
[agentskills.io best practices](https://agentskills.io/skill-creation/best-practices) and models them; `skills/reference/` holds three Apache-2.0 skills from
[anthropics/skills](https://github.com/anthropics/skills), copied unmodified (see its README).

`smoke` ends by having the model review `greet-user` with `skill-review`. The model has no file
tool, so the host passes `references/best-practices.md` in the prompt (the step the skill tells an
agent to take when judging), and the output schema has one required entry per rubric practice, so
the review is bounded and every finding is filed under the practice it judges. `llama3.1:8b`
completes it; models that stop early on long structured output may fail this step.
