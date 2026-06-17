# Local-LLM tooling

Rift's **Local LLM** workspace (kbd 4, off by default) shells out to the `claude`
CLI with `ANTHROPIC_BASE_URL` pointed at a local Anthropic `/v1/messages`-compatible
endpoint. As of **v0.19.0** the thinking-suppression shim is **baked into the Rust
backend** (`src-tauri/src/assistant/nothink.rs`) — there is no external proxy to run.

## Current stack (v0.19.0+)

```
CLI ──/v1/messages (+thinking)──▶ nothink.rs shim (in-process loopback)
                                    └ injects thinking:{type:disabled} ──▶ Ollama :11434
```

Just run Ollama; Rift handles the rest:

```sh
ollama serve            # :11434
```

In Rift → **Local LLM**: toggle on, base `http://localhost:11434` (raw Ollama),
**Detect** → pick the model (shipped default `qwen3.6-iq3-rift`), any non-empty key,
**Test connection** (green). **Optimize for Rift** bakes a `num_ctx`-bumped
`<model>-rift` variant so the prompt + MCP tool defs aren't truncated mid-turn.

Design doc: `docs/design/local-llm.md`. The superseded LiteLLM + Python
`strip_thinking_proxy.py` era (cont.127-129) lives in that doc's lineage sections and
in `git log`.

## `stress-test/`

The local-mode prompt-iteration harness — `harness-rift.mjs` faithfully replicates
Rift's local mode (exact tool surface + CLI name-rejections + Read-before-Write guard
+ workspace path-jail) running the LIVE addendum extracted from `turn.rs`. Use it
whenever you change `RIFT_SYSTEM_ADDENDUM_LOCAL`. See `stress-test/README.md` +
`stress-test/RESULTS.md`.
