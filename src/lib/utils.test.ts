import { describe, expect, it } from "vitest";
import { cn } from "./utils";

describe("cn", () => {
  it("keeps a color and a size on the same text- prefix", () => {
    expect(cn("text-fg", "text-sm")).toBe("text-fg text-sm");
    expect(cn("text-fg-muted", "text-base")).toBe("text-fg-muted text-base");
  });

  it("lets a later Rift color override an earlier one", () => {
    expect(cn("text-fg", "text-fg-muted")).toBe("text-fg-muted");
    expect(cn("bg-surface", "bg-surface-hover")).toBe("bg-surface-hover");
    expect(cn("border-border", "border-border-strong")).toBe("border-border-strong");
  });

  it("keeps a border color and a border width", () => {
    expect(cn("border-border", "border")).toBe("border-border border");
    expect(cn("ring-ring", "ring-2")).toBe("ring-ring ring-2");
  });

  it("resolves Rift-only theme names against their defaults", () => {
    expect(cn("rounded-md", "rounded-card")).toBe("rounded-card");
    expect(cn("rounded-sm", "rounded-island")).toBe("rounded-island");
    expect(cn("shadow-lg", "shadow-float")).toBe("shadow-float");
    expect(cn("ease-soft", "ease-page")).toBe("ease-page");
  });

  it("joins conditionals like clsx", () => {
    expect(cn("px-2", false && "px-4", { "font-semibold": true })).toBe("px-2 font-semibold");
  });
});
