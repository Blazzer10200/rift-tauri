import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock Tauri IPC before importing the store
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { connection } from "./connection.svelte.js";
import type { ServerProfile } from "./connection.svelte.js";

const makeServer = (overrides: Partial<ServerProfile> = {}): ServerProfile => ({
  key: "test-server",
  name: "Test",
  host: "10.0.0.1",
  port: 22,
  user: "root",
  keyPath: "/home/user/.ssh/id_rsa",
  remoteRoot: "/opt/fxserver",
  localRoot: "C:/Projects/fivem",
  ...overrides,
});

describe("connection.remoteForLocalPath()", () => {
  beforeEach(() => {
    connection.servers = [makeServer()];
    connection.selectedKey = "test-server";
  });

  it("returns null when no server is selected", () => {
    connection.selectedKey = null;
    expect(connection.remoteForLocalPath("C:/Projects/fivem/resources/mymod/fxmanifest.lua")).toBeNull();
  });

  it("maps a local path under localRoot to its remote equivalent", () => {
    const result = connection.remoteForLocalPath("C:/Projects/fivem/resources/mymod/fxmanifest.lua");
    expect(result).toBe("/opt/fxserver/resources/mymod/fxmanifest.lua");
  });

  it("returns null for a path outside localRoot", () => {
    expect(connection.remoteForLocalPath("C:/Other/file.lua")).toBeNull();
  });

  it("returns null for empty input", () => {
    expect(connection.remoteForLocalPath("")).toBeNull();
  });

  it("normalises backslashes in local path", () => {
    const result = connection.remoteForLocalPath("C:\\Projects\\fivem\\resources\\mymod\\fxmanifest.lua");
    expect(result).toBe("/opt/fxserver/resources/mymod/fxmanifest.lua");
  });

  it("is case-insensitive on the localRoot prefix (Windows paths)", () => {
    const result = connection.remoteForLocalPath("c:/projects/fivem/resources/mymod/init.lua");
    expect(result).toBe("/opt/fxserver/resources/mymod/init.lua");
  });
});
