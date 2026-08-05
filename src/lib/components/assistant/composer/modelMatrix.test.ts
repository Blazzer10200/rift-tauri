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
  fastModeInfoFor,
  modelAccessFor,
  openAiReasoningEffortsFor,
  settingsRowsFor,
  MODEL_OPTIONS,
  type ProviderAccessContext,
} from "./modelMatrix";
import type { CodexModel, OpenAiModel } from "../../../state/assistant/types";

const claude = MODEL_OPTIONS.find((model) => model.provider === "claude")!;
const openai = MODEL_OPTIONS.find((model) => model.provider === "openai")!;
const apiModel = (overrides: Partial<OpenAiModel> = {}): OpenAiModel => ({
  id: openai.id,
  label: openai.label,
  family: "gpt",
  contextWindow: null,
  description: "API model",
  reasoning: true,
  defaultReasoningEffort: "medium",
  supportedReasoningEfforts: ["none", "low", "medium", "high", "xhigh", "max"],
  fastMode: false,
  imageInput: true,
  available: true,
  ...overrides,
});
const codexModel = (overrides: Partial<CodexModel> = {}): CodexModel => ({
  id: openai.id as `gpt-${string}`,
  label: openai.label,
  description: "Subscription model",
  isDefault: true,
  defaultReasoningEffort: "medium",
  supportedReasoningEfforts: ["low", "medium"],
  serviceTiers: [],
  defaultServiceTier: null,
  upgradeModel: null,
  upgradeCopy: null,
  inputModalities: ["text", "image"],
  supportsPersonality: false,
  imageInput: true,
  ...overrides,
});
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
      openAiModels: [apiModel()],
    };
    expect(modelAccessFor(claude, openAiOnly).enabled).toBe(false);
    expect(modelAccessFor(openai, openAiOnly).enabled).toBe(true);
  });

  it("distinguishes subscription models from separately billed API fallback", () => {
    const codexOnly: ProviderAccessContext = {
      ...disconnected,
      codexReady: true,
      codexModels: [codexModel()],
    };
    expect(modelAccessFor(openai, codexOnly)).toMatchObject({
      enabled: true,
      tag: "Subscription",
    });

    const apiOnly: ProviderAccessContext = {
      ...disconnected,
      openAiConfigured: true,
      openAiModels: [apiModel()],
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
      openAiModels: [apiModel({ available: false })],
    };
    expect(modelAccessFor(openai, denied)).toMatchObject({ state: "unavailable", tag: "No access" });
    expect(modelAccessFor(openai, { ...denied, openAiModels: null, openAiError: "offline" }))
      .toMatchObject({ state: "error", tag: "Check failed" });
  });

  it("never crosses a chat's pinned billing route when selecting models", () => {
    const subscriptionOnly: ProviderAccessContext = {
      ...disconnected,
      codexReady: true,
      codexModels: [codexModel()],
      openAiConfigured: true,
      openAiModels: [apiModel({ available: false })],
    };
    expect(modelAccessFor(openai, subscriptionOnly, "codex").enabled).toBe(true);
    expect(modelAccessFor(openai, subscriptionOnly, "openai")).toMatchObject({
      enabled: false,
      tag: "No API access",
    });
  });
});

describe("effort capability", () => {
  it("renders the full X-High ladder for capable models", () => {
    expect(dialStopsFor(claude).map((stop) => stop.label)).toEqual(["Low", "Medium", "High", "X-High"]);
  });

  it("hides effort when live OpenAI metadata says reasoning is unsupported", () => {
    const caps = effortCapsFor(openai, [apiModel({ reasoning: false, supportedReasoningEfforts: [] })], "openai");
    expect(dialStopsFor(caps)).toEqual([]);
  });

  it("maps the exact live Codex capability set into Rift tiers", () => {
    const live = codexModel({
      id: "gpt-6-codex" as const,
      label: "GPT-6 Codex",
      description: "Future coding model",
      isDefault: false,
      defaultReasoningEffort: "medium",
      supportedReasoningEfforts: ["xhigh", "low", "medium", "future", "medium"],
      imageInput: true,
    });
    expect(codexReasoningEffortsFor(live)).toEqual(["none", "smart", "ultra"]);

    const model = chatGptModelsFor([live]).find((candidate) => candidate.id === live.id)!;
    expect(model).toMatchObject({ effort: true, maxEffort: "ultra", supportedEfforts: ["none", "smart", "ultra"] });
    expect(dialStopsFor(model).map((stop) => stop.label)).toEqual(["Low", "Medium", "X-High"]);
    expect(clampEffortForCaps("deep", model)).toBe("smart");
  });

  it("keeps API none and low distinct and exposes GPT max", () => {
    const live = apiModel();
    expect(openAiReasoningEffortsFor(live)).toEqual(["none", "low", "smart", "deep", "ultra", "max"]);
    const caps = effortCapsFor(openai, [live], "openai")!;
    expect(dialStopsFor(caps).map((stop) => stop.label)).toEqual(["None", "Low", "Medium", "High", "X-High", "Max"]);
  });

  it("exposes Codex max and ultra as separate top rungs", () => {
    const live = codexModel({
      supportedReasoningEfforts: ["low", "medium", "high", "xhigh", "max", "ultra"],
    });
    const model = chatGptModelsFor([live]).find((candidate) => candidate.id === live.id)!;
    expect(codexReasoningEffortsFor(live)).toEqual(["none", "smart", "deep", "ultra", "max", "agentic"]);
    expect(dialStopsFor({ ...model, offWireEffort: "low" }).map((stop) => stop.label))
      .toEqual(["Low", "Medium", "High", "X-High", "Max", "Ultra"]);
  });
});

describe("live ChatGPT model rows", () => {
  it("creates a complete row for an unknown live model and retains API fallbacks", () => {
    const live = codexModel({
      id: "gpt-6-codex-neo" as const,
      label: "GPT-6 Codex Neo",
      description: "Newest account coding model",
      isDefault: true,
      defaultReasoningEffort: "high",
      supportedReasoningEfforts: ["low", "high"],
      imageInput: true,
    });
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
    const live = codexModel({
      id: openai.id as `gpt-${string}`,
      label: "Uncurated backend label",
      description: "Live account description",
      isDefault: true,
      defaultReasoningEffort: "medium",
      supportedReasoningEfforts: ["low", "medium"],
      imageInput: false,
      inputModalities: ["text"],
    });
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

  it("uses live subscription tiers and API metadata for Fast availability", () => {
    const subscription = codexModel({
      id: "gpt-5.6-sol",
      serviceTiers: [{ id: "priority", name: "Fast", description: "1.5x speed" }],
    });
    const sol = chatGptModelsFor([subscription]).find((candidate) => candidate.id === subscription.id)!;
    expect(fastModeInfoFor(sol, "codex", [subscription], null)?.tag).toBe("2.5× credits");
    expect(fastModeInfoFor(sol, "openai", [subscription], [apiModel({ id: subscription.id, fastMode: true })])?.tag)
      .toBe("premium API");
    expect(fastModeInfoFor(sol, "codex", [codexModel({ id: subscription.id })], null)).toBeNull();
  });
});
