# skill-llm

Runnable examples of Agent Skills driving an LLM through
[turul-llm](https://github.com/aussierobots/turul-llm). Not published; examples only.

```sh
cargo run -p skill-llm --bin catalog   # the model picks a skill from the catalog, then uses it
cargo run -p skill-llm --bin author    # the model drafts a skill; it is validated, written, modified
cargo run -p skill-llm --bin smoke     # every public library capability, asserted step by step
```

All three run against an offline stub model by default, so `cargo test` needs no network.
For a live model:

| Variable | Effect |
|---|---|
| `RUN_OLLAMA_SMOKE=1` | Use Ollama at `http://localhost:11434` |
| `OLLAMA_BASE_URL` | Use Ollama at this URL (implies live mode) |
| `OLLAMA_MODEL` | Model name, default `llama3.1` |

`skills/` holds the example skills; `skills/reference/` holds three Apache-2.0 skills from
[anthropics/skills](https://github.com/anthropics/skills), copied unmodified (see its README).
