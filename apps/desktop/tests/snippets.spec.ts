import { describe, expect, it } from "vitest";
import type { SnippetInfo } from "../src/lib/api";
import { describeScope, initialValues, missingValues, needsForm, uniqueSnippetId } from "../src/lib/snippets";

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

  it("gives a new snippet an id from its name that is not taken", () => {
    expect(uniqueSnippetId("Git Status!", [])).toBe("git-status");
    expect(uniqueSnippetId("Git Status", ["git-status"])).toBe("git-status-2");
    expect(uniqueSnippetId("Git Status", ["git-status", "git-status-2"])).toBe("git-status-3");
    expect(uniqueSnippetId("???", [])).toBe("snippet");
  });

  it("says where a snippet applies", () => {
    const base = { id: "a", name: "A", command: "ls", description: null };
    expect(describeScope({ ...base, folder: null })).toBe("everywhere");
    expect(describeScope({ ...base, folder: "~/work" })).toBe("in ~/work");
  });
});
