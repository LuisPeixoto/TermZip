import { describe, expect, it } from "vitest";
import type { SnippetInfo } from "../src/lib/api";
import { initialValues, missingValues, needsForm } from "../src/lib/snippets";

const snippet = (parameters: SnippetInfo["parameters"]): SnippetInfo => ({
  id: "x",
  name: "X",
  description: null,
  parameters,
});

describe("snippets", () => {
  it("needs a form only when it asks for something", () => {
    expect(needsForm(snippet([]))).toBe(false);
    expect(needsForm(snippet([{ name: "a", default: null }]))).toBe(true);
  });

  it("starts the form with the defaults and empty fields for the rest", () => {
    const values = initialValues(
      snippet([
        { name: "branch", default: "main" },
        { name: "message", default: null },
        { name: "flags", default: "" },
      ]),
    );
    expect(values).toEqual({ branch: "main", message: "", flags: "" });
  });

  it("requires a value only where there is no default", () => {
    const asks = snippet([
      { name: "branch", default: "main" },
      { name: "message", default: null },
    ]);

    expect(missingValues(asks, { branch: "main", message: "" })).toEqual(["message"]);
    expect(missingValues(asks, { branch: "main", message: "   " })).toEqual(["message"]);
    expect(missingValues(asks, { branch: "", message: "fix" })).toEqual([]);
  });

  it("lets a parameter with a default be emptied on purpose", () => {
    expect(missingValues(snippet([{ name: "flags", default: "-l" }]), { flags: "" })).toEqual([]);
  });
});
