/** The last part of a path, for either kind of separator: `/a/b/` and `C:\a\b` both give `b`. */
export function basename(path: string | null | undefined): string | null {
  if (!path) return null;
  const trimmed = path.replace(/[\\/]+$/, "");
  if (!trimmed) return path; // the root itself
  const last = trimmed.split(/[\\/]/).pop();
  return last || path;
}
