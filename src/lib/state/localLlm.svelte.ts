// Ollama context-window machinery for the Models page (LocalLlmPage). The old
// local-mode toggle/config surface moved to the provider registry
// (providers.svelte.ts) — what's left here is the /api/show ctx probe + the
// one-click num_ctx optimize, which read the active wire fields backend-side.

import { invoke } from "@tauri-apps/api/core";

type LocalLlmDto = {
  base_url: string | null;
  model: string | null;
};

/** Ollama `/api/show` probe — effective context window for the configured model.
 *  `num_ctx === null` means it falls back to Ollama's 4096 default (the silent
 *  truncation that breaks agentic edits). `is_ollama === false` = non-Ollama
 *  endpoint (LiteLLM); the page then hides Ollama-specific guidance. */
type LocalCtxInfo = {
  is_ollama: boolean;
  model: string;
  num_ctx: number | null;
  max_ctx: number | null;
  params: string | null;
  quant: string | null;
  family: string | null;
};

/** Below this, Rift's prompt + tools + open files don't fit → the model loses
 *  its instructions mid-turn and stalls / refuses edits. Matches the backend
 *  optimize floor. */
const MIN_USABLE_CTX = 8192;

class LocalLlmStore {
  baseUrl = $state("");
  model = $state("");
  loaded = $state(false);
  /** Effective context window for the configured model (Ollama only). null until
   *  probed or when the endpoint isn't Ollama. */
  ctxInfo = $state<LocalCtxInfo | null>(null);
  ctxChecking = $state(false);
  optimizing = $state(false);

  /** True when we know the model is running on Ollama's tiny default context —
   *  the single biggest cause of local-mode stalls / refused edits. */
  get ctxUndersized(): boolean {
    const i = this.ctxInfo;
    if (!i || !i.is_ollama) return false;
    return i.num_ctx == null || i.num_ctx < MIN_USABLE_CTX;
  }

  async refresh() {
    try {
      const cfg = await invoke<LocalLlmDto>("assistant_get_local_llm_config");
      this.baseUrl = cfg.base_url ?? "";
      this.model = cfg.model ?? "";
    } catch (e) {
      console.error("load local-llm config failed", e);
    } finally {
      this.loaded = true;
    }
  }

  /** Probe Ollama's effective context window for the configured model, surfacing
   *  the 4096-default truncation that silently breaks agentic edits. Best-effort:
   *  clears ctxInfo on failure (no endpoint / not Ollama / unreachable) so the
   *  page just hides the guidance. */
  async refreshCtx() {
    if (!this.model.trim() || !this.baseUrl.trim()) {
      this.ctxInfo = null;
      return;
    }
    this.ctxChecking = true;
    try {
      this.ctxInfo = await invoke<LocalCtxInfo>("assistant_local_model_context");
    } catch (e) {
      console.error("local model context probe failed", e);
      this.ctxInfo = null;
    } finally {
      this.ctxChecking = false;
    }
  }

  /** One-click fix: bake a Rift-sized `num_ctx` variant of the configured model
   *  via Ollama `/api/create`, adopt it as the active model, and re-probe. The
   *  only lever for the 4096 default — Ollama can't take num_ctx per-request over
   *  the Anthropic adapter. `targetCtx` is clamped server-side to
   *  [8192, min(max,131072)]. Returns the new model name or the error to surface. */
  async optimize(targetCtx = 32768): Promise<{ ok: boolean; msg: string }> {
    this.optimizing = true;
    try {
      const variant = await invoke<string>("assistant_optimize_local_model", { targetCtx });
      this.model = variant;
      await this.refreshCtx();
      return { ok: true, msg: variant };
    } catch (e) {
      return { ok: false, msg: String(e) };
    } finally {
      this.optimizing = false;
    }
  }
}

export const localLlm = new LocalLlmStore();
