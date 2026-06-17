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

  /** Label for the in-chat pill — the configured model, else a generic tag. */
  get pillLabel(): string {
    return this.model.trim() || "Local model";
  }

  /** Who the composer is addressing — drives "Ask …" placeholders so they
   *  don't say "Claude" while local mode routes turns to a local model. */
  get askLabel(): string {
    return this.enabled ? "your local model" : "Claude";
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

  /** Round-trip a one-line prompt through the configured endpoint. Persists the
   *  current base + model first so the headless probe reads fresh config. */
  async test(): Promise<{ ok: boolean; msg: string }> {
    await this.saveBaseUrl();
    try {
      await this.saveModel();
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
    try {
      const reply = await invoke<string>("assistant_test_local_llm");
      return { ok: true, msg: reply };
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
  }
}

export const localLlm = new LocalLlmStore();
