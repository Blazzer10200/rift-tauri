import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { projects, projectRootKey } from "./projects.svelte.js";
import type { Project } from "./assistant/types";

const mockedInvoke = vi.mocked(invoke);

function makeProject(overrides: Partial<Project> = {}): Project {
  return {
    id: "id-1",
    name: "My Project",
    root: "C:/workspace/my-project",
    include: [],
    exclude: [],
    createdAt: Date.now(),
    ...overrides,
  };
}

beforeEach(() => {
  projects.items = [];
  projects.lastError = null;
  projects.loaded = false;
  projects.activeId = null;
  mockedInvoke.mockReset();
});

describe("projectRootKey", () => {
  it("lowercases the path", () => {
    expect(projectRootKey("C:/Workspace/MyProject")).toBe("c:/workspace/myproject");
  });

  it("strips trailing forward slash", () => {
    expect(projectRootKey("c:/workspace/proj/")).toBe("c:/workspace/proj");
  });

  it("strips trailing backslash", () => {
    expect(projectRootKey("c:\\workspace\\proj\\")).toBe("c:\\workspace\\proj");
  });

  it("strips multiple trailing slashes", () => {
    expect(projectRootKey("c:/workspace/proj///")).toBe("c:/workspace/proj");
  });

  it("strips mixed trailing slashes", () => {
    expect(projectRootKey("c:/workspace/proj/\\")).toBe("c:/workspace/proj");
  });

  it("returns empty string for null", () => {
    expect(projectRootKey(null)).toBe("");
  });

  it("returns empty string for undefined", () => {
    expect(projectRootKey(undefined)).toBe("");
  });

  it("returns empty string for empty string", () => {
    expect(projectRootKey("")).toBe("");
  });

  it("does not strip non-trailing slashes", () => {
    expect(projectRootKey("C:/foo/bar")).toBe("c:/foo/bar");
  });
});

describe("refresh()", () => {
  it("sets items and loaded=true and clears lastError on success", async () => {
    const p = makeProject();
    mockedInvoke.mockResolvedValueOnce([p]);

    await projects.refresh();

    expect(projects.items).toEqual([p]);
    expect(projects.loaded).toBe(true);
    expect(projects.lastError).toBe(null);
  });

  it("sets lastError to raw error string (no prefix) when invoke rejects", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("not a directory: C:/x"));

    await projects.refresh();

    expect(projects.lastError).toBe("Error: not a directory: C:/x");
    expect(projects.lastError).not.toContain("Load projects failed:");
    expect(projects.loaded).toBe(false);
  });

  it("does not update items on failure", async () => {
    const existing = makeProject({ id: "prior" });
    projects.items = [existing];
    mockedInvoke.mockRejectedValueOnce(new Error("boom"));

    await projects.refresh();

    expect(projects.items).toEqual([existing]);
  });

  it("clears lastError on subsequent success after a failure", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("fail"));
    await projects.refresh();
    expect(projects.lastError).not.toBe(null);

    mockedInvoke.mockResolvedValueOnce([]);
    await projects.refresh();
    expect(projects.lastError).toBe(null);
  });
});

describe("save()", () => {
  it("returns the id and clears lastError on success", async () => {
    const saved = makeProject({ id: "abc-123" });
    mockedInvoke.mockResolvedValueOnce([saved]);

    const id = await projects.save({
      id: "abc-123",
      name: "My Project",
      root: "C:/workspace",
      include: [],
      exclude: [],
    });

    expect(id).toBe("abc-123");
    expect(projects.lastError).toBe(null);
  });

  it("updates items on success", async () => {
    const saved = makeProject({ id: "abc-123", name: "Updated" });
    mockedInvoke.mockResolvedValueOnce([saved]);

    await projects.save({
      id: "abc-123",
      name: "Updated",
      root: "C:/workspace",
      include: [],
      exclude: [],
    });

    expect(projects.items).toEqual([saved]);
  });

  it("generates a random id when none is provided", async () => {
    mockedInvoke.mockResolvedValueOnce([makeProject()]);

    const id = await projects.save({
      name: "Auto ID",
      root: "C:/x",
      include: [],
      exclude: [],
    });

    expect(typeof id).toBe("string");
    expect(id!.length).toBeGreaterThan(0);
  });

  it("returns null and sets lastError (no prefix) on failure", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("disk full"));

    const id = await projects.save({
      name: "Bad",
      root: "C:/x",
      include: [],
      exclude: [],
    });

    expect(id).toBe(null);
    expect(projects.lastError).toBe("Error: disk full");
    expect(projects.lastError).not.toContain("Save project failed:");
  });

  it("clears a prior lastError on success (regression: stale error)", async () => {
    projects.lastError = "stale save error";
    mockedInvoke.mockResolvedValueOnce([makeProject()]);

    const id = await projects.save({ name: "Ok", root: "C:/y", include: [], exclude: [] });

    expect(id).not.toBe(null);
    expect(projects.lastError).toBe(null);
  });
});

describe("refresh() — loaded flag across success then failure", () => {
  it("a failed refresh after a prior success keeps loaded true (items unchanged)", async () => {
    mockedInvoke.mockResolvedValueOnce([makeProject()]);
    await projects.refresh();
    expect(projects.loaded).toBe(true);

    mockedInvoke.mockRejectedValueOnce(new Error("transient"));
    await projects.refresh();
    // loaded stays true from the prior good load; lastError carries the new error
    expect(projects.loaded).toBe(true);
    expect(projects.lastError).toBe("Error: transient");
    expect(projects.items).toHaveLength(1);
  });
});

describe("remove()", () => {
  it("updates items and clears lastError on success", async () => {
    const remaining = makeProject({ id: "other" });
    mockedInvoke.mockResolvedValueOnce([remaining]);

    await projects.remove("to-delete");

    expect(projects.items).toEqual([remaining]);
    expect(projects.lastError).toBe(null);
  });

  it("clears a prior lastError on success (regression: stale error)", async () => {
    projects.lastError = "some prior error";
    mockedInvoke.mockResolvedValueOnce([]);

    await projects.remove("any-id");

    expect(projects.lastError).toBe(null);
  });

  it("sets lastError (no prefix) on failure", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("not found"));

    await projects.remove("ghost-id");

    expect(projects.lastError).toBe("Error: not found");
  });

  it("clears activeId when the active project is removed", async () => {
    projects.activeId = "active-id";
    mockedInvoke.mockResolvedValueOnce([]);

    await projects.remove("active-id");

    expect(projects.activeId).toBe(null);
  });

  it("does not clear activeId when a different project is removed", async () => {
    projects.activeId = "active-id";
    mockedInvoke.mockResolvedValueOnce([]);

    await projects.remove("other-id");

    expect(projects.activeId).toBe("active-id");
  });
});

describe("byId()", () => {
  beforeEach(() => {
    projects.items = [
      makeProject({ id: "p1", name: "Alpha" }),
      makeProject({ id: "p2", name: "Beta" }),
    ];
  });

  it("returns the matching project", () => {
    expect(projects.byId("p1")?.name).toBe("Alpha");
    expect(projects.byId("p2")?.name).toBe("Beta");
  });

  it("returns null for unknown id", () => {
    expect(projects.byId("nope")).toBe(null);
  });

  it("returns null for null/undefined", () => {
    expect(projects.byId(null)).toBe(null);
    expect(projects.byId(undefined)).toBe(null);
  });
});

describe("byRoot()", () => {
  beforeEach(() => {
    projects.items = [
      makeProject({ id: "p1", root: "C:/workspace/alpha" }),
      makeProject({ id: "p2", root: "C:/workspace/beta/" }),
    ];
  });

  it("finds project by exact root (case-insensitive)", () => {
    expect(projects.byRoot("c:/workspace/alpha")?.id).toBe("p1");
    expect(projects.byRoot("C:/WORKSPACE/ALPHA")?.id).toBe("p1");
  });

  it("finds project ignoring trailing slash on stored root", () => {
    expect(projects.byRoot("c:/workspace/beta")?.id).toBe("p2");
  });

  it("finds project ignoring trailing slash on query", () => {
    expect(projects.byRoot("c:/workspace/alpha/")?.id).toBe("p1");
  });

  it("returns null for no match", () => {
    expect(projects.byRoot("c:/workspace/gamma")).toBe(null);
  });

  it("returns null for null/undefined/empty", () => {
    expect(projects.byRoot(null)).toBe(null);
    expect(projects.byRoot(undefined)).toBe(null);
    expect(projects.byRoot("")).toBe(null);
  });
});

describe("sorted getter", () => {
  it("sorts newest createdAt first", () => {
    const old = makeProject({ id: "old", name: "Old", createdAt: 1000 });
    const mid = makeProject({ id: "mid", name: "Mid", createdAt: 2000 });
    const newer = makeProject({ id: "new", name: "New", createdAt: 3000 });
    projects.items = [old, newer, mid];

    const ids = projects.sorted.map((p) => p.id);
    expect(ids).toEqual(["new", "mid", "old"]);
  });

  it("uses name as tiebreak when createdAt is equal", () => {
    const ts = 5000;
    const alpha = makeProject({ id: "a", name: "Alpha", createdAt: ts });
    const beta = makeProject({ id: "b", name: "Beta", createdAt: ts });
    const zeta = makeProject({ id: "z", name: "Zeta", createdAt: ts });
    projects.items = [zeta, beta, alpha];

    const names = projects.sorted.map((p) => p.name);
    expect(names).toEqual(["Alpha", "Beta", "Zeta"]);
  });

  it("does not mutate items array", () => {
    const p1 = makeProject({ id: "p1", createdAt: 1000 });
    const p2 = makeProject({ id: "p2", createdAt: 2000 });
    projects.items = [p1, p2];

    projects.sorted; // access the getter
    expect(projects.items[0].id).toBe("p1");
    expect(projects.items[1].id).toBe("p2");
  });
});
