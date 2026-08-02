// Unified settings-panel row builder — the pure glue both Composer (keyboard
// nav) and SettingsMenu (render) derive from. These lock the row ORDER
// contract; a drift here desyncs cursor from pixels.
import { describe, it, expect } from "vitest";
import {
  chatGptModelsFor,
  clampEffortForCaps,
  codexReasoningEffortsFor,
  dialStopsFor,
  effortCapsFor,
  modelAccessFor,
  settingsRowsFor,
  MODEL_OPTIONS,
  type ProviderAccessContext,
} from "./modelMatrix";

const claude = MODEL_OPTIONS.find((model) => model.provider === "claude")!;
const openai = MODEL_OPTIONS.find((model) => model.provider === "openai")!;
const disconnected: ProviderAccessContext = {
  claudeReady: false,
  claudeChecking: false,
  claudeError: null,
  claudeFree: false,
  codexReady: false,
  codexChecking: false,
  codexModels: null,
  codexError: null,
  openAiConfigured: false,
  openAiChecking: false,
  openAiModels: null,
  openAiError: null,
};

describe("settingsRowsFor", () => {
  it("model rows, then effort row when the dial applies", () => {
    const rows = settingsRowsFor({ dialApplies: true });
    expect(rows.slice(0, MODEL_OPTIONS.length).every((r) => r.kind === "model")).toBe(true);
    expect(rows[MODEL_OPTIONS.length]).toEqual({ kind: "effort" });
    expect(rows.length).toBe(MODEL_OPTIONS.length + 1);
  });

  it("no effort row when the dial doesn't apply", () => {
    const rows = settingsRowsFor({ dialApplies: false });
    expect(rows).toEqual(MODEL_OPTIONS.map((m) => ({ kind: "model", model: m })));
  });

  it("uses only selectable models for keyboard rows", () => {
    expect(settingsRowsFor({ dialApplies: true, models: [openai] })).toEqual([
      { kind: "model", model: openai },
      { kind: "effort" },
    ]);
  });
});

describe("model access", () => {
  it("locks both providers until their own connection is usable", () => {
    expect(modelAccessFor(claude, disconnected)).toMatchObject({ state: "setup", enabled: false });
    expect(modelAccessFor(openai, disconnected)).toMatchObject({ state: "setup", enabled: false });
  });

  it("keeps Claude and OpenAI access independent", () => {
    const claudeOnly = { ...disconnected, claudeReady: true };
    expect(modelAccessFor(claude, claudeOnly).enabled).toBe(true);
    expect(modelAccessFor(openai, claudeOnly).enabled).toBe(false);

    const openAiOnly = {
      ...disconnected,
      openAiConfigured: true,
      openAiModels: [{ id: openai.id, label: openai.label, family: "gpt", contextWindow: null, reasoning: true, imageInput: true, available: true }],
    };
    expect(modelAccessFor(claude, openAiOnly).enabled).toBe(false);
    expect(modelAccessFor(openai, openAiOnly).enabled).toBe(true);
  });

  it("distinguishes subscription models from separately billed API fallback", () => {
    const codexOnly: ProviderAccessContext = {
      ...disconnected,
      codexReady: true,
      codexModels: [{
        id: openai.id as `gpt-${string}`,
        label: openai.label,
        description: "Subscription model",
        isDefault: true,
        defaultReasoningEffort: "medium",
        supportedReasoningEfforts: ["low", "medium"],
        imageInput: true,
      }],
    };
    expect(modelAccessFor(openai, codexOnly)).toMatchObject({
      enabled: true,
      tag: "Subscription",
    });

    const apiOnly: ProviderAccessContext = {
      ...disconnected,
      openAiConfigured: true,
      openAiModels: [{
        id: openai.id, label: openai.label, family: "gpt", contextWindow: null,
        reasoning: true, imageInput: true, available: true,
      }],
    };
    expect(modelAccessFor(openai, apiOnly)).toMatchObject({
      enabled: true,
      tag: "API · separately billed",
    });
    expect(modelAccessFor(openai, apiOnly).detail).toContain("billed separately");
  });

  it("distinguishes account denial from a failed model probe", () => {
    const denied = {
      ...disconnected,
      openAiConfigured: true,
      openAiModels: [{ id: openai.id, label: openai.label, family: "gpt", contextWindow: null, reasoning: true, imageInput: true, available: false }],
    };
    expect(modelAccessFor(openai, denied)).toMatchObject({ state: "unavailable", tag: "No access" });
    expect(modelAccessFor(openai, { ...denied, openAiModels: null, openAiError: "offline" }))
      .toMatchObject({ state: "error", tag: "Check failed" });
  });
});

describe("effort capability", () => {
  it("renders the full X-High ladder for capable models", () => {
    expect(dialStopsFor(claude).map((stop) => stop.label)).toEqual(["Low", "Medium", "High", "X-High"]);
  });

  it("hides effort when live OpenAI metadata says reasoning is unsupported", () => {
    const caps = effortCapsFor(openai, [{
      id: openai.id, label: openai.label, family: "gpt", contextWindow: null,
      reasoning: false, imageInput: true, available: true,
    }]);
    expect(dialStopsFor(caps)).toEqual([]);
  });

  it("maps the exact live Codex capability set into Rift tiers", () => {
    const live = {
      id: "gpt-6-codex" as const,
      label: "GPT-6 Codex",
      description: "Future coding model",
      isDefault: false,
      defaultReasoningEffort: "medium",
      supportedReasoningEfforts: ["xhigh", "low", "medium", "future", "medium"],
      imageInput: true,
    };
    expect(codexReasoningEffortsFor(live)).toEqual(["none", "smart", "ultra"]);

    const model = chatGptModelsFor([live]).find((candidate) => candidate.id === live.id)!;
    expect(model).toMatchObject({ effort: true, maxEffort: "ultra", supportedEfforts: ["none", "smart", "ultra"] });
    expect(dialStopsFor(model).map((stop) => stop.label)).toEqual(["Low", "Medium", "X-High"]);
    expect(clampEffortForCaps("deep", model)).toBe("smart");
  });
});

describe("live ChatGPT model rows", () => {
  it("creates a complete row for an unknown live model and retains API fallbacks", () => {
    const live = {
      id: "gpt-6-codex-neo" as const,
      label: "GPT-6 Codex Neo",
      description: "Newest account coding model",
      isDefault: true,
      defaultReasoningEffort: "high",
      supportedReasoningEfforts: ["low", "high"],
      imageInput: true,
    };
    const models = chatGptModelsFor([live]);
    expect(models[0]).toMatchObject({
      id: live.id,
      label: live.label,
      version: "",
      tagline: live.description,
      blurb: "Text, vision & tools",
      provider: "openai",
      legacy: false,
      effort: true,
      maxEffort: "deep",
      supportedEfforts: ["none", "deep"],
    });
    expect(models[0].icon).toBeDefined();
    expect(models.filter((model) => model.id === live.id)).toHaveLength(1);
    for (const fallback of MODEL_OPTIONS.filter((model) => model.provider === "openai")) {
      expect(models.some((model) => model.id === fallback.id)).toBe(true);
    }
  });

  it("overlays curated presentation metadata while honoring live capabilities", () => {
    const live = {
      id: openai.id as `gpt-${string}`,
      label: "Uncurated backend label",
      description: "Live account description",
      isDefault: true,
      defaultReasoningEffort: "medium",
      supportedReasoningEfforts: ["low", "medium"],
      imageInput: false,
    };
    const model = chatGptModelsFor([live]).find((candidate) => candidate.id === openai.id)!;
    expect(model).toMatchObject({
      id: openai.id,
      label: openai.label,
      version: openai.version,
      icon: openai.icon,
      tagline: live.description,
      maxEffort: "smart",
      supportedEfforts: ["none", "smart"],
    });
    expect(dialStopsFor(model).map((stop) => stop.label)).toEqual(["Low", "Medium"]);
  });
});
