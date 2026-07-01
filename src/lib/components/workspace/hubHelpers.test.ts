import { describe, expect, it } from "vitest";
import { chatLastAt, pulseByRoot, relTime, type ChatLike } from "./hubHelpers";

const chat = (o: Partial<ChatLike>): ChatLike => ({
  id: "x",
  title: "t",
  messageCount: 1,
  createdAt: 1_000,
  costUsd: 0,
  ...o,
});

describe("relTime", () => {
  const now = 10_000_000_000;
  it("buckets seconds → weeks", () => {
    expect(relTime(now - 30_000, now)).toBe("just now");
    expect(relTime(now - 5 * 60_000, now)).toBe("5m ago");
    expect(relTime(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(relTime(now - 2 * 86_400_000, now)).toBe("2d ago");
    expect(relTime(now - 28 * 86_400_000, now)).toBe("4w ago");
  });
  it("clamps future timestamps to just now", () => {
    expect(relTime(now + 60_000, now)).toBe("just now");
  });
});

describe("chatLastAt", () => {
  it("prefers lastActivityAt, falls back to createdAt", () => {
    expect(chatLastAt(chat({ createdAt: 5, lastActivityAt: 9 }))).toBe(9);
    expect(chatLastAt(chat({ createdAt: 5 }))).toBe(5);
  });
});

describe("pulseByRoot", () => {
  it("rolls up chats, cost, and latest activity per canonical root", () => {
    // rootKey canonicalizes case + trailing separator (NOT \ vs / — the app
    // stores roots with consistent separators; see path.test.ts).
    const map = pulseByRoot([
      chat({ workspaceRoot: "C:/dev/App", costUsd: 1.5, lastActivityAt: 100 }),
      chat({ workspaceRoot: "c:/dev/app/", costUsd: 0.5, lastActivityAt: 300 }),
      chat({ workspaceRoot: "C:/other", costUsd: 2, createdAt: 50 }),
    ]);
    expect(map.get("c:/dev/app")).toEqual({ chats: 2, cost: 2, lastAt: 300 });
    expect(map.get("c:/other")).toEqual({ chats: 1, cost: 2, lastAt: 50 });
  });
  it("skips unfiled chats (no workspaceRoot)", () => {
    const map = pulseByRoot([chat({}), chat({ workspaceRoot: null })]);
    expect(map.size).toBe(0);
  });
});
