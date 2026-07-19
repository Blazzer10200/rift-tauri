import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";

// workspace.svelte.ts guards init() with `typeof window === "undefined"`.
// In the node test environment window is absent, so we stub it + localStorage
// before each test via vi.stubGlobal, then restore via vi.unstubAllGlobals.

const ACTIVE_KEY = "rift.ui.workspace.v1";

import { workspace } from "./workspace.svelte.js";

function makeFakeStorage() {
  const store = new Map<string, string>();
  return {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => { store.set(k, v); },
    removeItem: (k: string) => { store.delete(k); },
    clear: () => { store.clear(); },
  };
}

let fakeLS: ReturnType<typeof makeFakeStorage>;

const DEFAULT_ORDER = ["home", "chat", "projects", "settings", "ai-health"] as const;

beforeEach(() => {
  fakeLS = makeFakeStorage();
  vi.stubGlobal("window", {});
  vi.stubGlobal("localStorage", fakeLS);
  // Reset singleton rune state.
  workspace.activeId = "chat";
  workspace.everOpened = new Set();
  workspace.order = [...DEFAULT_ORDER];
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("init() — activeId resolution", () => {
  it("stored 'projects' folds to 'home' (migration)", () => {
    fakeLS.setItem(ACTIVE_KEY, "projects");
    workspace.init();
    expect(workspace.activeId).toBe("home");
    expect(fakeLS.getItem(ACTIVE_KEY)).toBe("home");
  });

  it("valid stored id 'settings' is preserved", () => {
    fakeLS.setItem(ACTIVE_KEY, "settings");
    workspace.init();
    expect(workspace.activeId).toBe("settings");
  });

  it("unknown/garbage stored id falls back to 'chat'", () => {
    fakeLS.setItem(ACTIVE_KEY, "garbage-workspace-xyz");
    workspace.init();
    expect(workspace.activeId).toBe("chat");
    expect(fakeLS.getItem(ACTIVE_KEY)).toBe("chat");
  });

  it("nothing stored defaults to 'chat'", () => {
    workspace.init();
    expect(workspace.activeId).toBe("chat");
    expect(fakeLS.getItem(ACTIVE_KEY)).toBe("chat");
  });
});

describe("init() — everOpened seeding", () => {
  it("everOpened contains activeId after init", () => {
    fakeLS.setItem(ACTIVE_KEY, "settings");
    workspace.init();
    expect(workspace.everOpened.has("settings")).toBe(true);
  });

  it("everOpened contains 'home' after projects→home migration", () => {
    fakeLS.setItem(ACTIVE_KEY, "projects");
    workspace.init();
    expect(workspace.everOpened.has("home")).toBe(true);
    expect(workspace.everOpened.has("projects")).toBe(false);
  });

  it("everOpened contains 'chat' on fresh init", () => {
    workspace.init();
    expect(workspace.everOpened.has("chat")).toBe(true);
  });
});

describe("setActive()", () => {
  beforeEach(() => {
    fakeLS.setItem(ACTIVE_KEY, "chat");
    workspace.init();
  });

  it("sets activeId", () => {
    workspace.setActive("settings");
    expect(workspace.activeId).toBe("settings");
  });

  it("adds id to everOpened (lazy-mount latch)", () => {
    expect(workspace.everOpened.has("settings")).toBe(false);
    workspace.setActive("settings");
    expect(workspace.everOpened.has("settings")).toBe(true);
  });

  it("everOpened retains previously opened ids when adding a new one", () => {
    workspace.setActive("settings");
    workspace.setActive("ai-health");
    expect(workspace.everOpened.has("settings")).toBe(true);
    expect(workspace.everOpened.has("ai-health")).toBe(true);
  });

  it("persists to localStorage", () => {
    workspace.setActive("ai-health");
    expect(fakeLS.getItem(ACTIVE_KEY)).toBe("ai-health");
  });

  it("calling setActive with already-open id keeps everOpened stable", () => {
    workspace.setActive("chat");
    const size = workspace.everOpened.size;
    workspace.setActive("chat");
    expect(workspace.everOpened.size).toBe(size);
  });
});

// Provider rip-out (2026-07-18): the "local-llm" Models workspace is retired.
// A persisted "local-llm" activeId must fold to "chat", not crash/dead-end.
describe("local-llm retired (provider rip-out)", () => {
  it("stored activeId 'local-llm' folds to 'chat' on init", () => {
    fakeLS.setItem(ACTIVE_KEY, "local-llm");
    workspace.init();
    expect(workspace.activeId).toBe("chat");
    expect(fakeLS.getItem(ACTIVE_KEY)).toBe("chat");
  });
});

const ORDER_KEY = "rift.ui.workspace-order.v1";

describe("init() — order restore + backfill", () => {
  it("restores a full stored order verbatim", () => {
    const stored = ["settings", "chat", "home", "ai-health", "projects"];
    fakeLS.setItem(ORDER_KEY, JSON.stringify(stored));
    workspace.init();
    expect(workspace.order).toEqual(stored);
  });

  it("backfills ids missing from an older stored order at their DEFAULT_ORDER-relative slot", () => {
    // A user persisted [settings, chat] before newer workspaces existed. Missing
    // ids must land at their default-relative position (not the end) so the
    // positional Ctrl+N switching matches the kbd hints.
    fakeLS.setItem(ORDER_KEY, JSON.stringify(["settings", "chat"]));
    workspace.init();
    expect(workspace.order).toEqual(["home", "projects", "settings", "chat", "ai-health"]);
  });

  it("filters unknown ids out of a stored order (and backfills the rest)", () => {
    fakeLS.setItem(ORDER_KEY, JSON.stringify(["terminal", "chat", "bogus", "home"]));
    workspace.init();
    // Unknown ids dropped; remaining relative order (chat before home) kept.
    expect(workspace.order.includes("terminal" as never)).toBe(false);
    expect(workspace.order.includes("bogus" as never)).toBe(false);
    expect([...workspace.order].sort()).toEqual([...DEFAULT_ORDER].sort());
    expect(workspace.order.indexOf("chat")).toBeLessThan(workspace.order.indexOf("home"));
  });

  it("unparseable ORDER_KEY leaves the default order (and warns, not throws)", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    fakeLS.setItem(ORDER_KEY, "{not json[");
    workspace.init();
    expect(workspace.order).toEqual([...DEFAULT_ORDER]);
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("non-array JSON in ORDER_KEY leaves the default order", () => {
    fakeLS.setItem(ORDER_KEY, JSON.stringify({ a: 1 }));
    workspace.init();
    expect(workspace.order).toEqual([...DEFAULT_ORDER]);
  });
});

describe("reorder() / resetOrder()", () => {
  it("moves an id and persists the new order", () => {
    workspace.reorder(1, 3); // chat → after settings
    expect(workspace.order).toEqual(["home", "projects", "settings", "chat", "ai-health"]);
    expect(JSON.parse(fakeLS.getItem(ORDER_KEY)!)).toEqual(workspace.order);
  });

  it("from === to and out-of-range from are no-ops (nothing persisted)", () => {
    workspace.reorder(2, 2);
    workspace.reorder(-1, 0);
    workspace.reorder(99, 0);
    expect(workspace.order).toEqual([...DEFAULT_ORDER]);
    expect(fakeLS.getItem(ORDER_KEY)).toBeNull();
  });

  it("out-of-range to clamps into bounds", () => {
    workspace.reorder(0, 99); // home → end
    expect(workspace.order[workspace.order.length - 1]).toBe("home");
  });

  it("resetOrder restores the default and clears the stored key", () => {
    workspace.reorder(0, 2);
    expect(fakeLS.getItem(ORDER_KEY)).not.toBeNull();
    workspace.resetOrder();
    expect(workspace.order).toEqual([...DEFAULT_ORDER]);
    expect(fakeLS.getItem(ORDER_KEY)).toBeNull();
  });
});

describe("migrateLegacy() — legacy shell keys (runs inside init)", () => {
  it("seeds activeId from the legacy right-pane key when the new key is absent", () => {
    fakeLS.setItem("rift.ui.right-pane.v1", "settings");
    workspace.init();
    expect(workspace.activeId).toBe("settings");
  });

  it("does NOT override an already-set new-shell activeId", () => {
    fakeLS.setItem(ACTIVE_KEY, "ai-health");
    fakeLS.setItem("rift.ui.right-pane.v1", "settings");
    workspace.init();
    expect(workspace.activeId).toBe("ai-health");
  });

  it("carries legacy activitybar order forward with chat forced first + backfill", () => {
    fakeLS.setItem("rift.ui.activitybar-order.v1", JSON.stringify(["settings", "home"]));
    workspace.init();
    expect(workspace.order[0]).toBe("chat");
    expect([...workspace.order].sort()).toEqual([...DEFAULT_ORDER].sort());
    expect(workspace.order.indexOf("settings")).toBeLessThan(workspace.order.indexOf("home"));
  });

  it("sweeps every legacy key (spot-check both eras)", () => {
    fakeLS.setItem("rift.ui.right-pane.v1", "settings");
    fakeLS.setItem("rift.terminal.savedTabs", "[]");
    fakeLS.setItem("rift.ui.dock-w.v1", "300");
    workspace.init();
    expect(fakeLS.getItem("rift.ui.right-pane.v1")).toBeNull();
    expect(fakeLS.getItem("rift.terminal.savedTabs")).toBeNull();
    expect(fakeLS.getItem("rift.ui.dock-w.v1")).toBeNull();
  });
});
