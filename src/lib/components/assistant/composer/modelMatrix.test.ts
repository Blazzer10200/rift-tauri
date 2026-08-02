// Unified settings-panel row builder — the pure glue both Composer (keyboard
// nav) and SettingsMenu (render) derive from. These lock the row ORDER
// contract; a drift here desyncs cursor from pixels.
import { describe, it, expect } from "vitest";
import {
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
});
