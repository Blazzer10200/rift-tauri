# Local-LLM proxy stack (experimental)

Run a local model through Rift's **Local LLM** workspace. Rift shells out to the
`claude` CLI with `ANTHROPIC_BASE_URL` pointed here, so the proxy just has to
speak the Anthropic `/v1/messages` API.

## Why two processes

The `claude` CLI sends an Anthropic `thinking` block on every turn (no flag
disables it). Non-thinking local models (e.g. `ollama/qwen3-coder:30b`) make
LiteLLM forward it to Ollama, which 500s with
`"qwen3-coder:30b" does not support thinking`. LiteLLM's `drop_params` can't
strip it on the `/v1/messages` adapter path, so a tiny shim removes it one hop
earlier.

```
CLI ──/v1/messages (+thinking)──▶ shim :4000 ──(thinking stripped)──▶ LiteLLM :4001 ──▶ Ollama :11434
```

Models that natively support `thinking` don't need the shim — point Rift
straight at LiteLLM on `:4000` and skip step 2.

## Run

```sh
# 1. LiteLLM behind the shim
pip install "litellm[proxy]"          # or: uv tool install litellm
litellm --config config.yaml --port 4001

# 2. Strip-thinking shim on :4000 (what Rift points at)
python strip_thinking_proxy.py 4000 4001
```

Then in Rift → **Local LLM**: base `http://localhost:4000`, **Detect** the
model, any non-empty key, **Test connection** (goes green).

## Files

- `strip_thinking_proxy.py` — stdlib reverse proxy; strips `thinking` /
  `context_management` / `output_config` from JSON bodies, streams everything
  else through (SSE-safe). Args: `[listen_port] [upstream_port]` (default 4000 4001).
- `config.yaml` — LiteLLM config with `drop_params` / `modify_params` (hygiene;
  the shim is what actually fixes qwen3-coder).
