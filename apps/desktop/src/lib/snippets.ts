/** What the palette needs to know about a snippet to ask for its values; the parsing itself is the backend's. */
import type { SnippetInfo } from "./api";

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
