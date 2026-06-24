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

beforeEach(() => {
  fakeLS = makeFakeStorage();
  vi.stubGlobal("window", {});
  vi.stubGlobal("localStorage", fakeLS);
  // Reset singleton rune state.
  workspace.activeId = "chat";
  workspace.everOpened = new Set();
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
    workspace.setActive("local-llm");
    expect(workspace.everOpened.has("settings")).toBe(true);
    expect(workspace.everOpened.has("local-llm")).toBe(true);
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
