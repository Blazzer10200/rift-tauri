// Multi-model provider registry (docs/design/multi-model-providers.md).
// Reactive mirror of the backend ProviderProfile list + the active selection.
// Activation copies the profile into the local-LLM wire fields backend-side;
// this store owns the fresh-session-on-switch UX the old localLlm toggle had.
// The legacy localLlm store stays for the Ollama ctx probe/optimize machinery.

import { invoke } from "@tauri-apps/api/core";
import { assistant } from "./assistant.svelte";
import { toast } from "./toast.svelte";

export type ProviderDto = {
  id: string;
  name: string;
  base_url: string;
  model: string | null;
  models: string[];
  preset: string | null;
  max_output_tokens: number | null;
  has_key: boolean;
  active: boolean;
};

/** What the upsert command takes — the DTO minus derived fields. */
export type ProviderDraft = {
  id: string;
  name: string;
  base_url: string;
  model: string | null;
  models: string[];
  preset: string | null;
  max_output_tokens: number | null;
};

export type PresetDef = {
  preset: string;
  name: string;
  base_url: string;
  models: string[];
  max_output_tokens: number | null;
  keyHint: string;
  desc: string;
};

// Endpoint bookmarks, verified 2026-07-16 against first-party docs (sources in
// docs/design/multi-model-providers.md). Model lists are DEFAULTS from those
// docs, not a catalog — free-text + Detect stay authoritative. OpenRouter's
// base is `/api` (its Anthropic endpoint is /api/v1/messages) and its
// /v1/models listing works; the three cloud presets don't serve a listing.
export const PRESETS: PresetDef[] = [
  {
    preset: "kimi",
    name: "Kimi — Moonshot",
    base_url: "https://api.moonshot.ai/anthropic",
    models: ["kimi-k3", "kimi-k2.7-code", "kimi-k2.7-code-highspeed"],
    max_output_tokens: 32768,
    keyHint: "Moonshot API key",
    desc: "Kimi K3 / K2.7 through Moonshot's Anthropic-compatible endpoint.",
  },
  {
    preset: "deepseek",
    name: "DeepSeek",
    base_url: "https://api.deepseek.com/anthropic",
    models: ["deepseek-v4-pro", "deepseek-v4-flash"],
    max_output_tokens: 32768,
    keyHint: "DeepSeek API key",
    desc: "DeepSeek V4 — also auto-maps Claude model names server-side.",
  },
  {
    preset: "glm",
    name: "GLM — Z.ai",
    base_url: "https://api.z.ai/api/anthropic",
    models: ["GLM-4.7", "GLM-4.5-Air"],
    max_output_tokens: 32768,
    keyHint: "Z.ai API key",
    desc: "GLM-4.7 / 4.5-Air through Z.ai's Anthropic-compatible endpoint.",
  },
  {
    preset: "openrouter",
    name: "OpenRouter",
    base_url: "https://openrouter.ai/api",
    models: [],
    max_output_tokens: 32768,
    keyHint: "OpenRouter API key",
    desc: "One key, the whole catalog (GPT, Gemini, Llama…). Use Detect to list models.",
  },
  {
    preset: "ollama",
    name: "Ollama (local)",
    base_url: "http://localhost:11434",
    models: [],
    max_output_tokens: null,
    keyHint: "any non-empty value",
    desc: "Local open-weights models — Ollama serves /v1/messages natively.",
  },
  {
    preset: "litellm",
    name: "LiteLLM proxy",
    base_url: "http://localhost:4000",
    models: [],
    max_output_tokens: null,
    keyHint: "proxy key (any value for most setups)",
    desc: "Self-hosted proxy — routes to anything LiteLLM supports.",
  },
];

/** Slugify a preset/custom name into a provider id the backend accepts. */
export function draftIdFor(preset: string | null, taken: Set<string>): string {
  const base = (preset ?? "custom").toLowerCase().replace(/[^a-z0-9-]+/g, "-").replace(/^-+|-+$/g, "") || "custom";
  if (!taken.has(base)) return base;
  for (let n = 2; n < 100; n++) {
    const id = `${base}-${n}`;
    if (!taken.has(id)) return id;
  }
  return `${base}-${Date.now() % 100000}`;
}

class ProvidersStore {
  list = $state<ProviderDto[]>([]);
  loaded = $state(false);

  get active(): ProviderDto | null {
    return this.list.find((p) => p.active) ?? null;
  }
  /** Composer glue — true when turns route to a provider instead of Claude. */
  get enabled(): boolean {
    return this.active != null;
  }
  get pillLabel(): string {
    const a = this.active;
    return a ? a.model?.trim() || a.name : "Model";
  }
  /** Who the composer is addressing — "Ask <model> anything" placeholders. */
  get askLabel(): string {
    const a = this.active;
    return a ? a.model?.trim() || a.name : "Claude";
  }
  get baseUrl(): string {
    return this.active?.base_url ?? "";
  }

  async refresh() {
    try {
      this.list = await invoke<ProviderDto[]>("assistant_list_providers");
    } catch (e) {
      console.error("list providers failed", e);
    } finally {
      this.loaded = true;
    }
  }

  /** Create/update a profile. Throws on backend validation failure. */
  async upsert(profile: ProviderDraft) {
    await invoke("assistant_upsert_provider", { profile });
    await this.refresh();
  }

  async remove(id: string) {
    await invoke("assistant_delete_provider", { id });
    await this.refresh();
  }

  async setKey(id: string, key: string | null) {
    await invoke("assistant_set_provider_key", { id, key: key?.trim() || null });
    await this.refresh();
  }

  /** Round-trip a one-line prompt through a profile (active or not). */
  async test(id: string): Promise<{ ok: boolean; msg: string; tokens?: number | null }> {
    try {
      const r = await invoke<{ reply: string; output_tokens: number | null }>("assistant_test_provider", { id });
      return { ok: true, msg: r.reply, tokens: r.output_tokens };
    } catch (e) {
      return { ok: false, msg: String(e) };
    }
  }

  /** Best-effort /v1/models probe; merges hits into the profile so the picker
   *  survives restarts. Returns the detected count (0 = endpoint lists nothing
   *  — normal for the Kimi/DeepSeek/GLM cloud bases). */
  async detectModels(id: string): Promise<number> {
    const p = this.list.find((x) => x.id === id);
    if (!p) return 0;
    let found: string[] = [];
    try {
      found = (await invoke<string[]>("assistant_list_provider_models", { id })).slice(0, 100);
    } catch (e) {
      console.error("detect provider models failed", e);
      return 0;
    }
    if (found.length > 0) {
      const merged = [...new Set([...p.models, ...found])].slice(0, 100);
      await this.upsert({
        id: p.id, name: p.name, base_url: p.base_url, model: p.model,
        models: merged, preset: p.preset, max_output_tokens: p.max_output_tokens,
      }).catch((e) => console.error("persist detected models failed", e));
    }
    return found.length;
  }

  /** Switch turns onto a provider (id) or back to Claude (null). A mid-chat
   *  switch resets to a fresh session — cloud and provider turns can't share
   *  one CLI session (different auth); the old chat flushes to History. */
  async activate(id: string | null) {
    const prev = this.active?.id ?? null;
    if (prev === id) return;
    await invoke("assistant_activate_provider", { id });
    await this.refresh();
    if (assistant.messages.length > 0) {
      await assistant.clearConversation();
      const name = id ? (this.active?.name ?? "provider") : "Claude";
      toast.push({
        severity: "info",
        title: id ? `Switched to ${name}` : "Switched back to Claude",
        detail: "Started a fresh chat — the previous one is saved in History.",
      });
    }
  }
}

export const providers = new ProvidersStore();
