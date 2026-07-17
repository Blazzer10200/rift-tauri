import { describe, expect, it } from "vitest";
import { ghDot, ghFixPrompt, ghRelTime, ghSyncLabel, type GhStatus } from "./githubHelpers";

const ok = (run: GhStatus["run"]): GhStatus => ({ state: "ok", branch: "main", repo: "o/r", run });

describe("ghDot", () => {
  it("is none when there is nothing to say", () => {
    expect(ghDot(null)).toBe("none");
    expect(ghDot({ state: "no_repo" })).toBe("none");
    expect(ghDot({ state: "not_github" })).toBe("none");
    expect(ghDot({ state: "no_gh" })).toBe("none");
    expect(ghDot({ state: "no_auth" })).toBe("none");
    expect(ghDot({ state: "error" })).toBe("none");
  });
  it("is idle for a GitHub repo with no runs", () => {
    expect(ghDot(ok(null))).toBe("idle");
    expect(ghDot(ok({ status: "completed", conclusion: "cancelled" }))).toBe("idle");
    expect(ghDot(ok({ status: "completed", conclusion: "skipped" }))).toBe("idle");
  });
  it("is busy while a run is in flight", () => {
    expect(ghDot(ok({ status: "in_progress", conclusion: null }))).toBe("busy");
    expect(ghDot(ok({ status: "queued", conclusion: null }))).toBe("busy");
    expect(ghDot(ok({ status: "completed", conclusion: "action_required" }))).toBe("busy");
  });
  it("maps conclusions to ok/err", () => {
    expect(ghDot(ok({ status: "completed", conclusion: "success" }))).toBe("ok");
    expect(ghDot(ok({ status: "completed", conclusion: "failure" }))).toBe("err");
    expect(ghDot(ok({ status: "completed", conclusion: "timed_out" }))).toBe("err");
    expect(ghDot(ok({ status: "completed", conclusion: "startup_failure" }))).toBe("err");
  });
});

describe("ghSyncLabel", () => {
  it("is null without upstream numbers", () => {
    expect(ghSyncLabel(null, null)).toBeNull();
    expect(ghSyncLabel(undefined, 2)).toBeNull();
  });
  it("phrases sync states", () => {
    expect(ghSyncLabel(0, 0)).toBe("In sync with origin");
    expect(ghSyncLabel(2, 0)).toBe("2 ahead");
    expect(ghSyncLabel(0, 3)).toBe("3 behind");
    expect(ghSyncLabel(2, 3)).toBe("2 ahead · 3 behind");
  });
});

describe("ghRelTime", () => {
  const now = new Date("2026-07-16T12:00:00Z").getTime();
  it("formats ages compactly", () => {
    expect(ghRelTime("2026-07-16T11:59:40Z", now)).toBe("just now");
    expect(ghRelTime("2026-07-16T11:45:00Z", now)).toBe("15m ago");
    expect(ghRelTime("2026-07-16T07:00:00Z", now)).toBe("5h ago");
    expect(ghRelTime("2026-07-13T12:00:00Z", now)).toBe("3d ago");
  });
  it("is empty on bad input", () => {
    expect(ghRelTime(undefined, now)).toBe("");
    expect(ghRelTime("not-a-date", now)).toBe("");
  });
});

describe("prompts", () => {
  it("fix prompt names the run and the tool", () => {
    const p = ghFixPrompt({ databaseId: 99, workflowName: "check", displayTitle: "fix: thing" });
    expect(p).toContain("run 99");
    expect(p).toContain("check — fix: thing");
    expect(p).toContain("gh_run_view");
    expect(p).not.toContain("failing job");
  });
  it("fix prompt pinpoints the failing job and step when known", () => {
    const p = ghFixPrompt({ databaseId: 7, workflowName: "release", failedJob: "build", failedStep: "vpk pack" });
    expect(p).toContain('The failing job is "build" at step "vpk pack".');
    const jobOnly = ghFixPrompt({ workflowName: "release", failedJob: "build" });
    expect(jobOnly).toContain('The failing job is "build".');
  });
});
