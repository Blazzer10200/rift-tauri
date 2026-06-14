# Local-LLM proxy stack (experimental)

Run a local model through Rift's **Local LLM** workspace. Rift shells out to the
`claude` CLI with `ANTHROPIC_BASE_URL` pointed here, so the proxy just has to
speak the Anthropic `/v1/messages` API.

## Why two processes

The `claude` CLI sends an Anthropic `thinking` block on every turn (no flag
disables it). Non-thinking local models (e.g. `ollama/qwen3-coder:30b`) make
LiteLLM forward it to Ollama, which 500s with
`"qwen3-coder:30b" does not support thinking`. LiteLLM's `drop_params` can't
strip it on the `/v1/messages` adapter path, so a tiny **thinking-aware** shim
handles it one hop earlier: it keeps `thinking` for thinking-capable models (so
their reasoning shows up as "Thought for Ns" in Rift) and drops it only for the
`NO_THINK_MARKERS` denylist (currently `coder`).

```
CLI ──/v1/messages (+thinking)──▶ shim :4000 ──▶ LiteLLM :4001 ──▶ Ollama :11434
                                    └ thinking dropped ONLY for no-think models
```

## Run

```sh
# 1. LiteLLM behind the shim
pip install "litellm[proxy]"          # or: uv tool install litellm
litellm --config config.yaml --port 4001

# 2. Strip-thinking shim on :4000 (what Rift points at)
python strip_thinking_proxy.py 4000 4001
```

Then in Rift → **Local LLM**: base `http://localhost:4000`, **Detect** the
model (default `ollama/qwen3:14b` — thinking + tools; `ollama/qwen3-coder:30b`
for heavier coding, no thinking), any non-empty key, **Test connection** (green).

## Files

- `strip_thinking_proxy.py` — stdlib reverse proxy; always strips
  `context_management` / `output_config`, strips `thinking` only for
  `NO_THINK_MARKERS` models (`coder`), streams everything else through (SSE-safe).
  Args: `[listen_port] [upstream_port]` (default 4000 4001).
- `config.yaml` — LiteLLM config: both models via `ollama_chat/` (native tools),
  `num_ctx: 32768` (Ollama's 4K default truncates Rift's prompt), `drop_params`.
