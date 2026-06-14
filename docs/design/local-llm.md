# Local-LLM support (experimental)

Status: **experimental, uncommitted** (cont.127, 2026-06-14). Testing/experiment only — not shipped, not behind a release.

## Core idea

Rift has **no inference engine** — it shells out to the `claude` CLI as a subprocess. The CLI honors `ANTHROPIC_BASE_URL` + `ANTHROPIC_API_KEY`, so pointing them at a local **Anthropic-Messages-API-compatible** server makes the whole turn pipeline (streaming, MCP tools, permissions) work unchanged. "Local mode" is therefore just *different env on the same spawn* — not a second code path.

A local server that speaks the Anthropic Messages API is required. Recommended: **LiteLLM proxy** in front of Ollama / llama.cpp / vLLM (exposes `/v1/messages`, translates tool calls). Ollama's own Anthropic endpoint works but tool-use fidelity is rough.

## Design: separate page + one gated spawn surface

All UX/config lives in a yankable 4th workspace; the model still flows through the single spawn path (`claude_command()` → `turn.rs`) via additive, flag-gated branches. **Flag off ⇒ spawn is byte-identical to the cloud path.**

## Touch points

**Backend**
- `secrets.rs` — `LOCAL_LLM_API_KEY` keychain constant (separate from `ASSISTANT_API_KEY` so it never shadows the cloud key).
- `config.rs` — fields `local_llm_enabled` / `local_llm_base_url` / `local_llm_model`; `is_valid_local_model_name` (same anti-flag-injection guard as `is_valid_model_name` but also allows `/` and `:` for `ollama/llama3:7b`-style names); `LocalLlmDto` (returns `has_key`, never the value); 5 get/set commands (trust_level pattern, `CONFIG_WRITE_LOCK`).
- `turn.rs` — **3 blocks, all `if cfg.local_llm_enabled`:**
  1. Model override + skip cloud model-pin + Fable guard (no thinking-block signatures to preserve).
  2. After the `use_api_key` block: force `--bare` (if not already), inject `ANTHROPIC_BASE_URL` + keychain key (`"local"` fallback). Local key/base override the api-key branch — local wins.
  3. `--effort` bypass (local models/proxies don't implement Anthropic thinking tiers).
  - Plus `use_full_config` (`:563`) is `&& !cfg.local_llm_enabled` — local forces `--bare`, so it disables piggyback for the same reason api-key mode does (keeps Rift's `--mcp-config` the strict source instead of a contradictory `--bare` + piggyback combo).
- `oneshot.rs` — two HTTP probes (NOT CLI spawns — a direct POST surfaces the real upstream status/body instead of a generic CLI timeout):
  - `assistant_test_local_llm`: POSTs `{base}/v1/messages` **with a `thinking` block** (mimics every real turn, so it fails the same way real traffic does instead of false-greening); returns the model reply or the upstream error verbatim.
  - `assistant_list_local_models`: GETs `{base}/v1/models`, returns the advertised model ids (filtered by `is_valid_local_model_name`) for the page's Detect picker. Key stays backend-side; only id strings cross to the renderer. Returns `[]` (not an error) on unreachable/empty.
- `lib.rs` — all 7 commands registered (glob-chained through `commands::assistant::*`).

**Frontend**
- `local-llm/LocalLlmPage.svelte` — 2-column bento (Mode + Connection left, Endpoint right) sized to fit the app's 800px min window height with no scrollbar. Toggle, base-URL, model (free-text + **Detect** picker populated from `/v1/models`, with a datalist), keychain key (`Configured` badge, never reveals), Test-connection button (green/red result), warning banner. Shared reactive store `state/localLlm.svelte.ts` (page + composer pill both read it).
- `state/workspace.svelte.ts` — `"local-llm"` added to `WorkspaceId` + `WORKSPACE_IDS`; the `init()` backfill auto-migrates existing users' persisted order.
- `components/workspaces/index.ts` — registry row (`Cpu` icon, kbd `4`).

## Decisions

- **`--bare` ON in local mode** — the one real unknown. `--bare` forces env-key auth so the CLI ignores OAuth/keychain. This is the documented way to make `ANTHROPIC_BASE_URL` + an env key take effect. **If a working proxy ever 401s, this is the line to revisit.**
- **`--effort` omitted entirely** (not sent as `low`).
- **Page inline at kbd 4**, not pinned-right like the Settings gear.
- **Mid-conversation toggle guard (cont.128):** flipping local mode with a non-empty chat calls `assistant.clearConversation()` (flushes the old chat to History) + toasts. Cloud/local can't share one CLI session (different auth + thinking-block signatures).
- **No temperature / context-limit controls.** The `claude` CLI exposes no such knobs (turn.rs forwards only `--model`/`--bare`/env and deliberately skips `--effort`), so those would be dead UI. Tune them on the proxy/Ollama side instead. Only the model picker is wired because it reflects a real endpoint capability (`/v1/models`).

## Caveats

- **Tool-calling is make-or-break — use `ollama_chat/`, not `ollama/` (cont.129).** The plain `ollama/` provider hits Ollama's `/api/generate` (text completion), where the model prints tool calls as raw JSON text (`{"name":"Read","arguments":{...}}`) that Rift can't execute — the agentic loop silently dies. `ollama_chat/` hits `/api/chat` (native function calling), so Ollama parses qwen3-coder's tool calls into real `tool_calls` that LiteLLM maps to Anthropic `tool_use`. config.yaml aliases the Rift-facing `ollama/qwen3-coder:30b` → `ollama_chat/qwen3-coder:30b` so the user-facing model name is unchanged. Verified in-app: "list the files here" → real `list_dir` execution → coherent answer. Still model-dependent; tiny models botch it regardless.
- The 2026-06-12 minimal-core strip removed "custom providers" to go first-party-only — this partially reintroduces that surface, but as a single flat toggle, not a CRUD provider list.

## Yank steps

Delete `LocalLlmPage.svelte`; remove the `"local-llm"` row in `index.ts` + the two entries in `workspace.svelte.ts`; delete the 3 `if cfg.local_llm_enabled` blocks in `turn.rs` (and the `&& !cfg.local_llm_enabled` on `:563`). Config fields/commands + the secrets const can stay harmlessly or be removed.

## The `thinking` param problem (cont.129 — the make-or-break)

The spawned `claude` CLI sends an Anthropic `thinking` block on **every** turn (interleaved-thinking beta — no CLI flag disables it; tried `--effort` omit, `MAX_THINKING_TOKENS=0`, `--settings alwaysThinkingEnabled:false`). Models that can't think (e.g. `ollama/qwen3-coder:30b`) make LiteLLM's Anthropic `/v1/messages` adapter forward `thinking`→Ollama's `think`, and Ollama rejects it at runtime:

```
HTTP 500  OllamaException - "qwen3-coder:30b" does not support thinking
```

LiteLLM's own `drop_params` / `additional_drop_params: ["thinking"]` do **not** fix this: `thinking` is statically *supported* by the ollama provider (other models think), so drop_params never strips it — the rejection is model-specific and runtime, and the experimental `/v1/messages` adapter doesn't honour `additional_drop_params` anyway (LiteLLM #8199).

**Fix: a thinking-aware strip-shim in front of LiteLLM.** `scripts/local-llm/strip_thinking_proxy.py` (stdlib only) listens on `:4000` and forwards method/path/headers + the streaming response to LiteLLM on `:4001`. It always drops `context_management` / `output_config` (Claude-orchestration noise), but drops `thinking` **only for models that can't think** — matched by a substring denylist (`NO_THINK_MARKERS = ("coder",)`). Topology:

```
CLI ──/v1/messages+thinking──▶ shim :4000 ──▶ LiteLLM :4001 ──▶ Ollama :11434
                                  └ thinking dropped ONLY for no-think models (qwen3-coder)
```

Both streaming (SSE turns) and non-streaming (the probe) verified through it.

### Thinking restored for thinking-capable models (cont.129b)

Default model is now **`ollama/qwen3:14b`** (thinking-capable, native tools, fits 16GB VRAM). Because the shim no longer strips `thinking` for it, the CLI's `thinking` block flows through → Ollama `think:true` → the model reasons → LiteLLM maps the reasoning into an Anthropic `thinking` content block (with signature) → **Rift renders it as an expandable "Thought for Ns" section.** Verified in-app: bat-and-ball trap ($0.05) and the 8-balls/2-weighings puzzle (3ⁿ → 27) both solved correctly with visible thinking.

`num_ctx: 32768` is set per-model in config.yaml — Ollama's ~4K default truncates Rift's large system prompt + MCP tool defs and wrecks tool use. It's a provider-specific param, passed straight through (not dropped).

## To run

```
# 1. LiteLLM behind the shim (drop_params is still set as hygiene — see config.yaml)
pip install "litellm[proxy]"          # or: uv tool install litellm
litellm --config scripts/local-llm/config.yaml --port 4001

# 2. The strip-thinking shim on :4000 (what Rift points at)
python scripts/local-llm/strip_thinking_proxy.py 4000 4001
```
Local LLM page → base `http://localhost:4000`, **Detect** → pick the model (default `ollama/qwen3:14b` for thinking + tools; `ollama/qwen3-coder:30b` for heavier coding without thinking), any key → **Test connection** (goes green).

> The shim handles both model classes on one endpoint (`:4000`): it keeps `thinking` for thinking-capable models so their reasoning displays, and strips it for the `coder` denylist so they don't 500. Add other no-think model markers to `NO_THINK_MARKERS` in the shim if needed.
