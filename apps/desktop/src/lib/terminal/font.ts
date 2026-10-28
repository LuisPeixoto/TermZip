/** The monospace families tried, in order, when no font is chosen. */
export const DEFAULT_FONT_FAMILY =
  '"JetBrains Mono", "SF Mono", "Cascadia Code", Menlo, Consolas, "Liberation Mono", monospace';

/** The font stack for a chosen family (or none): what the terminal and the previews are drawn with. */
export function fontStack(family: string | null | undefined): string {
  return family?.trim() ? family : DEFAULT_FONT_FAMILY;
}
