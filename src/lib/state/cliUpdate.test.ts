import { describe, it, expect, beforeEach, vi } from "vitest";

// cliUpdate.svelte.ts imports `invoke` for runUpdate(); mock it so importing
// the module (and constructing instances) never touches the Tauri bridge. The
// tests below exercise only the pure detection/semver/summary logic, so the
// mock never actually fires.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { CliUpdate } from "./cliUpdate.svelte.js";

// Fresh instance per test — the real export is a singleton, but the detection
// methods read mutable `$state` (latest/dismissed/method/...), so isolation
// keeps one case from bleeding into the next.
let cu: CliUpdate;
beforeEach(() => {
  cu = new CliUpdate();
});

describe("isNewer / semver compare", () => {
  it("true only when latest is strictly newer than installed", () => {
    cu.latest = "2.1.5";
    expect(cu.isNewer("2.1.4")).toBe(true);
    expect(cu.isNewer("2.0.99")).toBe(true);
    expect(cu.isNewer("1.9.9")).toBe(true);
  });

  it("false at equal or older, never a downgrade nag", () => {
    cu.latest = "2.1.5";
    expect(cu.isNewer("2.1.5")).toBe(false);
    expect(cu.isNewer("2.1.6")).toBe(false);
    expect(cu.isNewer("3.0.0")).toBe(false);
  });

  it("tolerates a leading v and trailing noise", () => {
    cu.latest = "2.1.111 (Claude Code)";
    expect(cu.isNewer("v2.1.110")).toBe(true);
    expect(cu.isNewer("2.1.111")).toBe(false);
  });

  it("compares minor/patch independently of magnitude (no lexical bug)", () => {
    cu.latest = "2.1.10";
    expect(cu.isNewer("2.1.9")).toBe(true); // 10 > 9 numerically, not "10" < "9"
  });

  it("false when either side is missing or unparseable", () => {
    cu.latest = "2.1.5";
    expect(cu.isNewer(null)).toBe(false);
    expect(cu.isNewer("not-a-version")).toBe(false);
    cu.latest = null;
    expect(cu.isNewer("2.1.4")).toBe(false);
  });
});

describe("available — newer AND not dismissed", () => {
  beforeEach(() => {
    cu.latest = "2.1.5";
  });
  it("true when newer and undismissed", () => {
    expect(cu.available("2.1.4")).toBe(true);
  });
  it("false once the current latest is dismissed", () => {
    cu.dismissed = "2.1.5";
    expect(cu.available("2.1.4")).toBe(false);
  });
  it("re-surfaces when a newer version supersedes the dismissed one", () => {
    cu.dismissed = "2.1.4"; // user dismissed an older release
    expect(cu.available("2.1.3")).toBe(true);
  });
});

describe("isAnyStale — multi-install drift", () => {
  beforeEach(() => {
    cu.latest = "2.1.5";
  });
  it("true if ANY install is behind", () => {
    expect(cu.isAnyStale([{ version: "2.1.5" }, { version: "2.1.0" }], null)).toBe(true);
  });
  it("false when every install is current", () => {
    expect(cu.isAnyStale([{ version: "2.1.5" }, { version: "2.1.5" }], null)).toBe(false);
  });
  it("falls back to the single active version when no installs list", () => {
    expect(cu.isAnyStale(null, "2.1.0")).toBe(true);
    expect(cu.isAnyStale([], "2.1.5")).toBe(false);
  });
  it("ignores installs with no detected version", () => {
    expect(cu.isAnyStale([{ version: null }], "2.1.0")).toBe(false);
  });
  it("false when latest is unknown", () => {
    cu.latest = null;
    expect(cu.isAnyStale([{ version: "1.0.0" }], "1.0.0")).toBe(false);
  });
});

describe("availableAny — stale AND not dismissed", () => {
  it("gates the multi-install path on dismissal too", () => {
    cu.latest = "2.1.5";
    expect(cu.availableAny([{ version: "2.1.0" }], null)).toBe(true);
    cu.dismissed = "2.1.5";
    expect(cu.availableAny([{ version: "2.1.0" }], null)).toBe(false);
  });
});

describe("commandFor / updateCommand", () => {
  it("native installs self-update; everything else uses npm @latest", () => {
    expect(cu.commandFor("native")).toBe("claude update");
    expect(cu.commandFor("npm")).toBe("npm install -g @anthropic-ai/claude-code@latest");
    expect(cu.commandFor("unknown")).toBe("npm install -g @anthropic-ai/claude-code@latest");
    expect(cu.commandFor(null)).toBe("npm install -g @anthropic-ai/claude-code@latest");
  });
  it("updateCommand routes by the synced method", () => {
    cu.setMethod("native");
    expect(cu.updateCommand).toBe("claude update");
    cu.setMethod("npm");
    expect(cu.updateCommand).toBe("npm install -g @anthropic-ai/claude-code@latest");
  });
  it("a STUCK native install gets the reinstall command, not `claude update` again", () => {
    cu.setMethod("native");
    cu.updateStuck = true;
    expect(cu.updateCommand).toBe(cu.reinstallCommand);
    expect(cu.updateCommand).not.toBe("claude update");
  });
  it("a stuck npm install still gets the npm @latest command", () => {
    cu.setMethod("npm");
    cu.updateStuck = true;
    expect(cu.updateCommand).toBe("npm install -g @anthropic-ai/claude-code@latest");
  });
  it("reinstallCommand is one of the documented native installers", () => {
    expect([
      "irm https://claude.ai/install.ps1 | iex",
      "curl -fsSL https://claude.ai/install.sh | bash",
    ]).toContain(cu.reinstallCommand);
  });
});

describe("summary — contextual line precedence", () => {
  it("update error outranks everything (danger)", () => {
    cu.updateError = "npm exploded";
    cu.updateStuck = true;
    const s = cu.summary([{ version: "1" }, { version: "2" }]);
    expect(s.tone).toBe("danger");
    expect(s.headline).toBe("Update failed");
    expect(s.detail).toBe("npm exploded");
  });
  it("stuck-after-update (npm/unknown) is a warning that says it didn't change", () => {
    cu.updateStuck = true;
    expect(cu.summary(null).tone).toBe("warn");
    expect(cu.summary(null).headline).toBe("Still behind after update");
  });
  it("stuck native reframes as restart-to-apply, not broken", () => {
    cu.setMethod("native");
    cu.updateStuck = true;
    const s = cu.summary(null);
    expect(s.tone).toBe("warn");
    expect(s.headline).toBe("Restart to finish updating");
    expect(s.detail).toContain("next time Claude Code starts");
  });
  it("multi-install reports the count", () => {
    const s = cu.summary([{ version: "1" }, { version: "2" }]);
    expect(s.tone).toBe("accent");
    expect(s.detail).toContain("2 claude installs");
  });
  it("single native install mentions background auto-update", () => {
    cu.setMethod("native");
    expect(cu.summary([{ version: "1" }]).detail).toContain("auto-update");
  });
  it("default single-install line", () => {
    const s = cu.summary(null);
    expect(s.tone).toBe("accent");
    expect(s.headline).toBe("Update available");
    expect(s.detail).toContain("newer claude CLI");
  });
});

describe("versionUnreadable — active version can't be read (#42)", () => {
  it("true when installs exist but the active version is null", () => {
    expect(cu.versionUnreadable([{ version: null }], null)).toBe(true);
    expect(cu.versionUnreadable([{ version: "2.1.0" }, { version: null }], null)).toBe(true);
  });
  it("false when the active version reads fine", () => {
    expect(cu.versionUnreadable([{ version: "2.1.0" }], "2.1.0 (Claude Code)")).toBe(false);
  });
  it("false with no detected installs — onboarding owns the no-CLI surface", () => {
    expect(cu.versionUnreadable([], null)).toBe(false);
    expect(cu.versionUnreadable(null, null)).toBe(false);
  });
});

describe("checkFailedPersistently — quiet banner gate (#42)", () => {
  type Internals = { _retries: number; _retryTimer: ReturnType<typeof setTimeout> | null };
  it("false on a fresh failure — the retry ladder may still heal it", () => {
    cu.status = "error";
    expect(cu.checkFailedPersistently).toBe(false);
  });
  it("true only once retries are exhausted and none is pending", () => {
    cu.status = "error";
    (cu as unknown as Internals)._retries = 99;
    expect(cu.checkFailedPersistently).toBe(true);
  });
  it("false while a retry is still scheduled, and false after recovery", () => {
    cu.status = "error";
    const internals = cu as unknown as Internals;
    internals._retries = 99;
    internals._retryTimer = setTimeout(() => {}, 60_000);
    expect(cu.checkFailedPersistently).toBe(false);
    clearTimeout(internals._retryTimer);
    internals._retryTimer = null;
    cu.status = "ok";
    expect(cu.checkFailedPersistently).toBe(false);
  });
});

describe("dismiss", () => {
  it("pins the dismissed marker to the current latest", () => {
    cu.latest = "2.1.5";
    cu.dismiss();
    expect(cu.dismissed).toBe("2.1.5");
  });
  it("no-ops when no latest is known", () => {
    cu.latest = null;
    cu.dismiss();
    expect(cu.dismissed).toBe(null);
  });
});
