import { describe, it, expect, beforeEach } from "vitest";
import { vi } from "vitest";

// Mock Tauri IPC before importing the store. sync-page imports `invoke` at
// module load; the test only touches the in-memory derivations (`groups`,
// `totals`, dismissShrunk, etc.) so we never call into IPC.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { syncPage } from "./sync-page.svelte.js";
import type { DriftBucket, DriftEntry, AbortedShrunkFolder } from "./sync-page.svelte.js";

const makeEntry = (
  resource_name: string,
  rel_path: string,
  bucket: DriftBucket,
  reason = "test",
): DriftEntry => ({
  resource_name,
  rel_path,
  local_path: `C:/local/${resource_name}/${rel_path}`,
  remote_path: `/remote/${resource_name}/${rel_path}`,
  bucket,
  local_exists: bucket !== "to_pull" && bucket !== "to_delete",
  remote_exists: bucket !== "to_push" && bucket !== "to_delete_remote",
  local_size: 100,
  remote_size: 100,
  local_mtime: null,
  remote_mtime: null,
  has_snapshot: false,
  reason,
});

describe("syncPage.totals", () => {
  beforeEach(() => {
    syncPage.entries = [];
    syncPage.abortedShrunk = [];
    syncPage.dismissedShrunk = new Set();
  });

  it("returns zero counts for empty entries", () => {
    expect(syncPage.totals).toEqual({
      push: 0, pull: 0, del: 0, delRemote: 0, conf: 0, total: 0,
    });
  });

  it("counts each bucket independently", () => {
    syncPage.entries = [
      makeEntry("a", "1.lua", "to_push"),
      makeEntry("a", "2.lua", "to_push"),
      makeEntry("a", "3.lua", "to_pull"),
      makeEntry("b", "4.lua", "to_delete"),
      makeEntry("b", "5.lua", "to_delete_remote"),
      makeEntry("b", "6.lua", "conflict"),
      makeEntry("b", "7.lua", "synced"), // synced does not count toward total
    ];
    expect(syncPage.totals).toEqual({
      push: 2, pull: 1, del: 1, delRemote: 1, conf: 1, total: 6,
    });
  });

  it("synced bucket never contributes to total", () => {
    syncPage.entries = [
      makeEntry("a", "1.lua", "synced"),
      makeEntry("a", "2.lua", "synced"),
      makeEntry("a", "3.lua", "synced"),
    ];
    expect(syncPage.totals.total).toBe(0);
  });
});

describe("syncPage.groups", () => {
  beforeEach(() => {
    syncPage.entries = [];
  });

  it("returns no groups when entries is empty", () => {
    expect(syncPage.groups).toEqual([]);
  });

  it("groups entries by resource_name and counts pending per bucket", () => {
    syncPage.entries = [
      makeEntry("alpha", "1.lua", "to_push"),
      makeEntry("alpha", "2.lua", "to_push"),
      makeEntry("alpha", "3.lua", "to_pull"),
      makeEntry("beta", "10.lua", "conflict"),
    ];
    const g = syncPage.groups;
    expect(g).toHaveLength(2);
    const alpha = g.find((r) => r.resource === "alpha")!;
    const beta = g.find((r) => r.resource === "beta")!;
    expect(alpha.to_push).toHaveLength(2);
    expect(alpha.to_pull).toHaveLength(1);
    expect(alpha.total_pending).toBe(3);
    expect(beta.conflict).toHaveLength(1);
    expect(beta.total_pending).toBe(1);
  });

  it("sorts groups by total_pending descending", () => {
    syncPage.entries = [
      makeEntry("small", "1.lua", "to_push"),
      makeEntry("big", "1.lua", "to_push"),
      makeEntry("big", "2.lua", "to_push"),
      makeEntry("big", "3.lua", "to_push"),
      makeEntry("medium", "1.lua", "to_push"),
      makeEntry("medium", "2.lua", "to_push"),
    ];
    const g = syncPage.groups;
    expect(g.map((r) => r.resource)).toEqual(["big", "medium", "small"]);
  });

  it("excludes resources whose entries are all synced (total_pending === 0)", () => {
    syncPage.entries = [
      makeEntry("synced-only", "1.lua", "synced"),
      makeEntry("synced-only", "2.lua", "synced"),
      makeEntry("dirty", "1.lua", "to_push"),
    ];
    const g = syncPage.groups;
    expect(g.map((r) => r.resource)).toEqual(["dirty"]);
  });

  it("counts to_delete_remote as pending work", () => {
    syncPage.entries = [
      makeEntry("a", "1.lua", "to_delete_remote"),
    ];
    const g = syncPage.groups;
    expect(g).toHaveLength(1);
    expect(g[0].to_delete_remote).toHaveLength(1);
    expect(g[0].total_pending).toBe(1);
  });
});

describe("syncPage shrunk-folder dismissal", () => {
  const mkAborted = (remote_root: string): AbortedShrunkFolder => ({
    resource_name: remote_root.split("/").pop() ?? "x",
    remote_root,
    baseline_count: 100,
    listing_count: 5,
  });

  beforeEach(() => {
    syncPage.abortedShrunk = [];
    syncPage.dismissedShrunk = new Set();
  });

  it("visibleAbortedShrunk filters out dismissed folders", () => {
    syncPage.abortedShrunk = [
      mkAborted("/r/keep"),
      mkAborted("/r/drop"),
    ];
    syncPage.dismissShrunk("/r/drop");
    const v = syncPage.visibleAbortedShrunk;
    expect(v).toHaveLength(1);
    expect(v[0].remote_root).toBe("/r/keep");
  });

  it("dismissShrunk creates a new Set (immutable reactivity)", () => {
    syncPage.abortedShrunk = [mkAborted("/r/x")];
    const before = syncPage.dismissedShrunk;
    syncPage.dismissShrunk("/r/x");
    const after = syncPage.dismissedShrunk;
    expect(after).not.toBe(before);
    expect(after.has("/r/x")).toBe(true);
  });

  it("returns empty visible list when every folder dismissed", () => {
    syncPage.abortedShrunk = [
      mkAborted("/r/a"),
      mkAborted("/r/b"),
    ];
    syncPage.dismissShrunk("/r/a");
    syncPage.dismissShrunk("/r/b");
    expect(syncPage.visibleAbortedShrunk).toEqual([]);
  });
});
