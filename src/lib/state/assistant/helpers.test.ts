import { afterEach, describe, expect, it, vi } from "vitest";
import {
  FABLE_DISABLED, FABLE_SUNSET_MS, clampEffort, effortToFlag, fableAvailable,
  fastEligible, flattenToolResult, isStaleTurnEpoch, loadEffort,
  migrateThinkingPins, modelFamily, previewToolInput, ctxWindowForModelId,
  modelNativeWindow, planContextCap, autoCompactTriggerTokens,
} from "./helpers";

afterEach(() => vi.useRealTimers());

describe("context window (model × plan)", () => {
  const MAX = planContextCap("max"); // 1M
  const FREE = planContextCap("free"); // 200K

  it("plan caps: free=200K, pro & max=1M", () => {
    expect(planContextCap("free")).toBe(200_000);
    expect(planContextCap("pro")).toBe(1_000_000);
    expect(planContextCap("max")).toBe(1_000_000);
  });

  it("native window resolves the BARE aliases Rift sends to the CLI (regression: read 200K → gauge 5× too full)", () => {
    expect(modelNativeWindow("opus")).toBe(1_000_000);
    expect(modelNativeWindow("sonnet")).toBe(1_000_000);
    expect(modelNativeWindow("fable")).toBe(1_000_000);
  });

  it("native window: haiku 200K, full pinned ids + [1m] suffix 1M, unknown 200K", () => {
    expect(modelNativeWindow("haiku")).toBe(200_000);
    expect(modelNativeWindow("claude-opus-4-8")).toBe(1_000_000);
    expect(modelNativeWindow("claude-sonnet-4-6")).toBe(1_000_000);
    expect(modelNativeWindow("claude-opus-4-8[1m]")).toBe(1_000_000);
    // The backend appends `[1m]` to the CLI --model arg for 1M-gated Sonnets
    // (config.rs::cli_model_arg) and the CLI echoes it back in lastModelId — the
    // gauge must read these as 1M so it agrees with the CLI's auto-compaction
    // threshold (the "compacting at 14%" bug this fixes).
    expect(modelNativeWindow("claude-sonnet-5[1m]")).toBe(1_000_000);
    expect(modelNativeWindow("claude-sonnet-4-6[1m]")).toBe(1_000_000);
    expect(modelNativeWindow("some-future-model")).toBe(200_000);
    expect(modelNativeWindow(null)).toBe(200_000);
  });

  it("plan cap clamps the native window: Free on Opus = 200K, Max on Opus = 1M", () => {
    expect(ctxWindowForModelId("opus", FREE)).toBe(200_000);
    expect(ctxWindowForModelId("opus", MAX)).toBe(1_000_000);
    expect(ctxWindowForModelId("sonnet", FREE)).toBe(200_000);
  });

  it("omitting the plan cap returns the raw native window (no clamp)", () => {
    expect(ctxWindowForModelId("opus")).toBe(1_000_000);
    expect(ctxWindowForModelId("haiku")).toBe(200_000);
  });

  it("never exceeds the model's native window even on a generous plan (Haiku stays 200K on Max)", () => {
    expect(ctxWindowForModelId("haiku", MAX)).toBe(200_000);
  });
});

describe("fableAvailable", () => {
  it("manual kill-switch forces unavailable regardless of date", () => {
    // Fable pulled 2026-06-14 (US-gov disablement). While FABLE_DISABLED the
    // gate is false even before the date sunset; this test self-heals when the
    // flag flips back to re-enable.
    if (!FABLE_DISABLED) return;
    vi.useFakeTimers();
    vi.setSystemTime(FABLE_SUNSET_MS - 1);
    expect(fableAvailable()).toBe(false);
  });

  it("flips at the Jun 22 2026 EOD-UTC sunset when enabled", () => {
    if (FABLE_DISABLED) return; // kill-switch overrides the date gate
    vi.useFakeTimers();
    vi.setSystemTime(FABLE_SUNSET_MS - 1);
    expect(fableAvailable()).toBe(true);
    vi.setSystemTime(FABLE_SUNSET_MS);
    expect(fableAvailable()).toBe(false);
  });
});

describe("migrateThinkingPins", () => {
  const installLS = (seed: Record<string, string>) => {
    const store = new Map(Object.entries(seed));
    const ls = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
      key: (i: number) => Array.from(store.keys())[i] ?? null,
      get length() { return store.size; },
    };
    vi.stubGlobal("localStorage", ls);
    return store;
  };
  afterEach(() => vi.unstubAllGlobals());

  it("clears every per-folder thinkingEnabled pin but leaves model/effort pins + global baseline", () => {
    const store = installLS({
      "rift.assistant.thinkingEnabled::C:/proj/a": "on",
      "rift.assistant.thinkingEnabled::C:/proj/b": "off",
      "rift.assistant.thinkingEnabled": "off",          // global baseline — keep
      "rift.assistant.thinkingEffort::C:/proj/a": "deep", // effort pin — keep
      "rift.assistant.model::C:/proj/a": "opus",          // model pin — keep
    });
    migrateThinkingPins();
    expect(store.has("rift.assistant.thinkingEnabled::C:/proj/a")).toBe(false);
    expect(store.has("rift.assistant.thinkingEnabled::C:/proj/b")).toBe(false);
    expect(store.get("rift.assistant.thinkingEnabled")).toBe("off");
    expect(store.get("rift.assistant.thinkingEffort::C:/proj/a")).toBe("deep");
    expect(store.get("rift.assistant.model::C:/proj/a")).toBe("opus");
    expect(store.get("rift.assistant.thinkingPinSweep.v1")).toBe("done");
  });

  it("is idempotent — a fresh on-pin written after the sweep survives a second run", () => {
    const store = installLS({ "rift.assistant.thinkingEnabled::C:/proj/a": "on" });
    migrateThinkingPins(); // sweeps the stale pin, marks done
    store.set("rift.assistant.thinkingEnabled::C:/proj/a", "on"); // user re-pins intentionally
    migrateThinkingPins(); // must NOT sweep again
    expect(store.get("rift.assistant.thinkingEnabled::C:/proj/a")).toBe("on");
  });
});

describe("loadEffort — legacy quick-tier migration (retired 2026-07-03)", () => {
  const installLS = (seed: Record<string, string>) => {
    const store = new Map(Object.entries(seed));
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
    });
    return store;
  };
  afterEach(() => vi.unstubAllGlobals());

  it("folds a stored 'quick' baseline into 'smart' (same medium wire flag)", () => {
    installLS({ "rift.assistant.thinkingEffort": "quick" });
    expect(loadEffort()).toBe("smart");
  });

  it("folds a per-workspace 'quick' pin into 'smart' without touching valid tiers", () => {
    installLS({
      "rift.assistant.thinkingEffort::C:/proj/a": "quick",
      "rift.assistant.thinkingEffort": "deep",
    });
    expect(loadEffort("C:/proj/a")).toBe("smart"); // legacy pin coerced
    expect(loadEffort()).toBe("deep");             // baseline untouched
  });

  it("still rejects garbage → default 'smart'", () => {
    installLS({ "rift.assistant.thinkingEffort": "bogus" });
    expect(loadEffort()).toBe("smart");
  });
});

describe("modelFamily", () => {
  it("maps every selector to its aurora family", () => {
    expect(modelFamily("haiku")).toBe("haiku");
    expect(modelFamily("opus")).toBe("opus");
    expect(modelFamily("claude-opus-4-7")).toBe("opus");
    expect(modelFamily("claude-fable-5")).toBe("opus");
    expect(modelFamily("sonnet")).toBe("sonnet");
  });
});

describe("fastEligible (must mirror config.rs model_fast_eligible)", () => {
  it("is Opus-family only — Fable shares the opus VISUAL family but is not fast-eligible", () => {
    expect(fastEligible("opus")).toBe(true);
    expect(fastEligible("claude-opus-4-8")).toBe(true);
    expect(fastEligible("claude-opus-4-7")).toBe(true);
    expect(fastEligible("claude-fable-5")).toBe(false);
    expect(fastEligible("sonnet")).toBe(false);
    expect(fastEligible("claude-sonnet-5")).toBe(false);
    expect(fastEligible("haiku")).toBe(false);
  });
});

describe("effortToFlag (must mirror src-tauri assistant turn.rs mapping)", () => {
  it("suppresses effort entirely on haiku", () => {
    expect(effortToFlag("deep", "haiku")).toBeNull();
  });
  it("maps none/smart/deep/ultra to low/medium/high/xhigh", () => {
    expect(effortToFlag("none", "opus")).toBe("low");
    expect(effortToFlag("smart", "opus")).toBe("medium"); // responsive default
    expect(effortToFlag("deep", "opus")).toBe("high");
    expect(effortToFlag("ultra", "claude-fable-5")).toBe("xhigh");
  });
  it("clamps an out-of-range tier to the model ceiling before mapping", () => {
    // Sonnet 5 reaches ultra(xhigh) — an ultra pref passes straight through.
    expect(effortToFlag("ultra", "sonnet")).toBe("xhigh");
    // smart on Sonnet is the responsive default → medium (not the old high).
    expect(effortToFlag("smart", "sonnet")).toBe("medium");
  });
});

describe("clampEffort (model effort ceiling)", () => {
  it("leaves Sonnet 5/Opus/Fable at ultra (all reach xhigh)", () => {
    expect(clampEffort("ultra", "sonnet")).toBe("ultra"); // Sonnet 5 accepts xhigh
    expect(clampEffort("deep", "sonnet")).toBe("deep");  // in range
    expect(clampEffort("smart", "sonnet")).toBe("smart"); // already in range
    expect(clampEffort("ultra", "opus")).toBe("ultra");
    expect(clampEffort("ultra", "claude-fable-5")).toBe("ultra");
  });
  it("floors Haiku to none (rejects effort wholesale)", () => {
    expect(clampEffort("ultra", "haiku")).toBe("none");
  });
});

describe("effortToFlag — default tier maps to medium (responsive interactive default)", () => {
  it("smart → medium; none → low", () => {
    expect(effortToFlag("smart", "opus")).toBe("medium");
    expect(effortToFlag("none", "opus")).toBe("low");
  });
  it("deep → high, ultra → xhigh on a model that reaches them", () => {
    expect(effortToFlag("deep", "opus")).toBe("high");
    expect(effortToFlag("ultra", "opus")).toBe("xhigh");
  });
  it("Sonnet 5 reaches deep(high) and ultra(xhigh)", () => {
    expect(effortToFlag("deep", "sonnet")).toBe("high");
    expect(effortToFlag("ultra", "sonnet")).toBe("xhigh"); // Sonnet 5 accepts xhigh
  });
  it("haiku rejects effort wholesale → null", () => {
    expect(effortToFlag("smart", "haiku")).toBeNull();
    expect(effortToFlag("deep", "haiku")).toBeNull();
  });
});

describe("flattenToolResult + previewToolInput", () => {
  it("flattens strings, text-part arrays, and rejects the rest", () => {
    expect(flattenToolResult("plain")).toBe("plain");
    expect(flattenToolResult([{ text: "a" }, { other: 1 }, { text: "b" }])).toBe("ab");
    expect(flattenToolResult({ not: "array" })).toBe("");
  });
  it("image parts render an honest [image] placeholder, not silence", () => {
    expect(flattenToolResult([{ type: "image", source: { data: "…" } }])).toBe("[image]");
    expect(flattenToolResult([{ text: "before " }, { type: "image" }, { text: " after" }])).toBe("before [image] after");
  });
  it("previews the first known field in priority order and caps at 120", () => {
    expect(previewToolInput("Bash", { command: "ls", file_path: "x" })).toBe("ls");
    expect(previewToolInput("Read", { file_path: "src/a.ts" })).toBe("src/a.ts");
    expect(previewToolInput("X", { command: "c".repeat(150) })).toBe("c".repeat(120) + "…");
    expect(previewToolInput("X", {})).toBeNull();
    expect(previewToolInput("X", undefined)).toBeNull();
  });
});

describe("isStaleTurnEpoch (#80 stale-event discrimination)", () => {
  it("flags a mismatched positive epoch as stale — both directions", () => {
    expect(isStaleTurnEpoch(3, 2)).toBe(true); // old turn's terminal after a new send
    expect(isStaleTurnEpoch(2, 3)).toBe(true); // defensive: future epoch is equally not-ours
  });
  it("accepts the live turn's own epoch", () => {
    expect(isStaleTurnEpoch(3, 3)).toBe(false);
  });
  it("accepts epoch-less payloads (prewarm/legacy/dev hot-reload)", () => {
    expect(isStaleTurnEpoch(3, undefined)).toBe(false);
    expect(isStaleTurnEpoch(3, null)).toBe(false);
    expect(isStaleTurnEpoch(3, 0)).toBe(false);
    expect(isStaleTurnEpoch(3, "2")).toBe(false);
  });
  it("accepts anything while the tab has no epoch yet (fresh store, no send this app-life)", () => {
    expect(isStaleTurnEpoch(0, 5)).toBe(false);
  });
});

describe("autoCompactTriggerTokens (user auto-compact tuning → effective trigger)", () => {
  it("defaults to ~90% of the window when nothing is configured (probe absent or all-null)", () => {
    expect(autoCompactTriggerTokens(null, 200_000)).toBe(180_000);
    expect(autoCompactTriggerTokens({ enabled: true, windowTokens: null, pct: null }, 1_000_000)).toBe(900_000);
  });
  it("honors a tuned-down window + pct (the 250K-on-1M repro)", () => {
    const cfg = { enabled: true, windowTokens: 312_500, pct: 80 };
    expect(autoCompactTriggerTokens(cfg, 1_000_000)).toBe(250_000);
  });
  it("clamps a configured window larger than the real one", () => {
    const cfg = { enabled: true, windowTokens: 1_000_000, pct: 80 };
    expect(autoCompactTriggerTokens(cfg, 200_000)).toBe(160_000);
  });
  it("applies the default fraction to a window override without a pct", () => {
    const cfg = { enabled: true, windowTokens: 312_500, pct: null };
    expect(autoCompactTriggerTokens(cfg, 1_000_000)).toBe(281_250);
  });
  it("returns null when the user disabled auto-compact, or without a window", () => {
    expect(autoCompactTriggerTokens({ enabled: false, windowTokens: null, pct: null }, 1_000_000)).toBeNull();
    expect(autoCompactTriggerTokens(null, 0)).toBeNull();
  });
});

