// Experimental local-LLM mode (cont.127). Shared reactive surface for the
// Local LLM workspace + the composer's local-mode pill, so toggling on the
// settings page lights up the in-chat indicator live. Yankable: delete this
// file + its two importers (LocalLlmPage, Composer's local-pill block).

import { invoke } from "@tauri-apps/api/core";
import { assistant } from "./assistant.svelte";
import { toast } from "./toast.svelte";

type LocalLlmDto = {
  enabled: boolean;
  base_url: string | null;
  model: string | null;
  has_key: boolean;
};

/** Ollama `/api/show` probe — effective context window for the configured model.
 *  `num_ctx === null` means it falls back to Ollama's 4096 default (the silent
 *  truncation that breaks agentic edits). `is_ollama === false` = non-Ollama
 *  endpoint (LiteLLM); the page then hides Ollama-specific guidance. */
export type LocalCtxInfo = {
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
export const MIN_USABLE_CTX = 8192;

class LocalLlmStore {
  enabled = $state(false);
  baseUrl = $state("");
  model = $state("");
  hasKey = $state(false);
  loaded = $state(false);
  /** Models advertised by the endpoint's /v1/models, for the picker. Empty
   *  until detected; the model field stays free-text so custom names work. */
  models = $state<string[]>([]);
  detecting = $state(false);
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

  /** Label for the in-chat pill — the configured model, else a generic tag. */
  get pillLabel(): string {
    return this.model.trim() || "Local model";
  }

  /** Who the composer is addressing — drives "Ask …" placeholders so they
   *  don't say "Claude" while local mode routes turns to a local model. */
  get askLabel(): string {
    return this.enabled ? this.pillLabel : "Claude";
  }

  async refresh() {
    try {
      const cfg = await invoke<LocalLlmDto>("assistant_get_local_llm_config");
      this.enabled = cfg.enabled;
      this.baseUrl = cfg.base_url ?? "";
      this.model = cfg.model ?? "";
      this.hasKey = cfg.has_key;
    } catch (e) {
      console.error("load local-llm config failed", e);
    } finally {
      this.loaded = true;
    }
  }

  /** Flip local mode. Reverts optimistic state on failure. When the flip lands
   *  mid-conversation it resets the active chat to a fresh session — cloud and
   *  local turns can't share one CLI session (different auth + thinking-block
   *  signatures), so continuing in place would mix them. The old chat is
   *  flushed to History first, nothing is lost. */
  async setEnabled(value: boolean) {
    const prev = this.enabled;
    if (prev === value) return;
    this.enabled = value;
    try {
      await invoke("assistant_set_local_llm_enabled", { value });
    } catch (e) {
      console.error("set local-llm enabled failed", e);
      this.enabled = prev;
      throw e;
    }
    if (assistant.messages.length > 0) {
      await assistant.clearConversation();
      toast.push({
        severity: "info",
        title: value ? "Switched to local mode" : "Switched back to Claude",
        detail: "Started a fresh chat — the previous one is saved in History.",
      });
    }
  }

  async saveBaseUrl() {
    try {
      await invoke("assistant_set_local_llm_base_url", { value: this.baseUrl.trim() || null });
    } catch (e) {
      console.error("set local-llm base_url failed", e);
    }
  }

  /** Persist the model. Throws on backend validation failure so the page can
   *  surface the rejected name (anti-flag-injection guard lives server-side). */
  async saveModel() {
    await invoke("assistant_set_local_llm_model", { value: this.model.trim() || null });
  }

  async saveKey(key: string | null) {
    await invoke("assistant_set_local_llm_key", { key: key?.trim() || null });
    this.hasKey = !!key?.trim();
  }

  /** Ask the endpoint what models it serves (GET /v1/models, key stays
   *  backend-side). Persists the base URL first so the probe reads fresh
   *  config. Auto-selects the only model when the field is empty. Returns the
   *  count so the caller can surface it; swallows failures (picker is optional).*/
  async listModels(): Promise<number> {
    this.detecting = true;
    try {
      await this.saveBaseUrl();
      this.models = await invoke<string[]>("assistant_list_local_models");
      if (!this.model.trim() && this.models.length === 1) {
        this.model = this.models[0];
        await this.saveModel().catch(() => {});
      }
      return this.models.length;
    } catch (e) {
      console.error("list local models failed", e);
      this.models = [];
      return 0;
    } finally {
      this.detecting = false;
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

  /** Round-trip a one-line prompt through the configured endpoint. Persists the
   *  current base + model first so the headless probe reads fresh config.
   *  `tokens` = output-token count (when the endpoint reports usage), paired
   *  with the caller's measured round-trip ms for an approximate tok/s. */
  async test(): Promise<{ ok: boolean; msg: string; tokens?: number | null }> {
    await this.saveBaseUrl();
    try {
      await this.saveModel();
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
    try {
      const r = await invoke<{ reply: string; output_tokens: number | null }>("assistant_test_local_llm");
      return { ok: true, msg: r.reply, tokens: r.output_tokens };
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
  }
}

export const localLlm = new LocalLlmStore();
