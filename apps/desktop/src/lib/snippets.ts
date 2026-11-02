/** What the palette needs to know about a snippet to ask for its values; the parsing itself is the backend's. */
import type { Snippet, SnippetInfo } from "./api";
import { slug } from "./profiles";

/** Mirrors `snippet.rs`; the backend still validates, this only gives an earlier, kinder message. */
export const MAX_SNIPPETS = 128;

/** An id for a new snippet that is not taken. */
export function uniqueSnippetId(name: string, taken: Iterable<string>): string {
  const used = new Set(taken);
  const base = slug(name) || "snippet";
  let id = base;
  for (let number = 2; used.has(id); number += 1) id = `${base}-${number}`;
  return id;
}

/** The line under a snippet in the list: where it applies, or that it applies everywhere. */
export const describeScope = (snippet: Snippet): string => (snippet.folder ? `in ${snippet.folder}` : "everywhere");

/** A snippet that asks for values is filled in before it is typed; the others are typed at once. */
export const needsForm = (snippet: SnippetInfo): boolean => snippet.parameters.length > 0;

/** The form starts with each parameter's default, or with an empty field. */
export function initialValues(snippet: SnippetInfo): Record<string, string> {
  return Object.fromEntries(snippet.parameters.map((parameter) => [parameter.name, parameter.default ?? ""]));
}

/**
 * The parameters that cannot be left blank: the ones with no default. A parameter with a default may be
 * emptied on purpose (`ls {{flags:-l}}` with no flags), but a required one left empty is a slip.
 */
export function missingValues(snippet: SnippetInfo, values: Record<string, string>): string[] {
  return snippet.parameters
    .filter((parameter) => parameter.default === null && (values[parameter.name] ?? "").trim() === "")
    .map((parameter) => parameter.name);
}
