import { describe, expect, it } from "vitest";
import { parseAskQuestions } from "./askQuestions";

describe("parseAskQuestions", () => {
  it("parses the canonical schema shape", () => {
    const out = parseAskQuestions({
      questions: [
        {
          question: "Which approach?",
          header: "Approach",
          multiSelect: true,
          options: [{ label: "A", description: "fast" }, { label: "B" }],
        },
      ],
    });
    expect(out).toEqual([
      {
        question: "Which approach?",
        header: "Approach",
        multiSelect: true,
        options: [{ label: "A", description: "fast" }, { label: "B", description: undefined }],
      },
    ]);
  });

  it("coerces plain-string options", () => {
    const out = parseAskQuestions({
      questions: [{ question: "Pick?", options: ["Ship now", "  Wait  ", ""] }],
    });
    expect(out[0].options).toEqual([{ label: "Ship now" }, { label: "Wait" }]);
    expect(out[0].header).toBe("");
    expect(out[0].multiSelect).toBe(false);
  });

  it("parses questions sent as a JSON-encoded string (the live 2026-07-28 shape)", () => {
    const out = parseAskQuestions({
      questions: '[{"question": "What do you want to do?", "options": ["Ship", "Verify first"]}]',
    });
    expect(out).toHaveLength(1);
    expect(out[0].options.map((o) => o.label)).toEqual(["Ship", "Verify first"]);
  });

  it("accepts a flat top-level {question, options} call and a bare object", () => {
    expect(parseAskQuestions({ question: "Ship it?", options: [{ label: "Yes" }, "No"] })).toEqual([
      { question: "Ship it?", header: "", multiSelect: false, options: [{ label: "Yes", description: undefined }, { label: "No" }] },
    ]);
    expect(parseAskQuestions({ questions: { question: "One?", options: ["A"] } })[0].question).toBe("One?");
  });

  it("keeps an option-less question for freeform answering, drops fully-empty entries", () => {
    const out = parseAskQuestions({ questions: [{ question: "Thoughts?" }, { options: [42] }, null] });
    expect(out).toEqual([{ question: "Thoughts?", header: "", multiSelect: false, options: [] }]);
  });

  it("returns [] for absent or garbage input", () => {
    expect(parseAskQuestions(undefined)).toEqual([]);
    expect(parseAskQuestions({})).toEqual([]);
    expect(parseAskQuestions({ questions: "not json" })).toEqual([]);
    expect(parseAskQuestions({ questions: 7 })).toEqual([]);
  });
});
