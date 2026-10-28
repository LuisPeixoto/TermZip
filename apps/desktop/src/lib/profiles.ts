import type { Profile } from "./api";
import { LIMITS } from "./settings";

/** Mirrors the limits in `profile.rs`; the backend still validates, these only give an earlier, kinder message. */
export const MAX_PROFILES = 32;
export const MAX_ENV_VARS = 64;
export const MAX_ENV_KEY_CHARS = 128;
export const MAX_ENV_VALUE_BYTES = 4 * 1024;

/** The built-in profile: the top-level shell and folder of the preferences. */
export const BUILT_IN_ID = "default";

const RESERVED = new Set(["TERM", "COLORTERM", "TERM_PROGRAM", "TERM_PROGRAM_VERSION", "ZDOTDIR"]);

export function isReservedEnv(key: string): boolean {
  const upper = key.toUpperCase();
  return RESERVED.has(upper) || upper.startsWith("TERMZIP_");
}

/** `Work Laptop!` becomes `work-laptop`. */
export function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** An id for a new profile that is not taken and is not the built-in one. */
export function uniqueProfileId(name: string, taken: Iterable<string>): string {
  const used = new Set(taken);
  let base = slug(name);
  if (!base || base === BUILT_IN_ID) base = "profile";
  let id = base;
  for (let number = 2; used.has(id); number += 1) id = `${base}-${number}`;
  return id;
}

export type EnvResult = { env: Record<string, string> } | { error: string };

/** Reads `KEY=value` lines. Blank lines and `#` comments are skipped; the first `=` splits. */
export function parseEnv(text: string): EnvResult {
  const env: Record<string, string> = {};
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line.trim() || line.trim().startsWith("#")) continue;
    const at = line.indexOf("=");
    const where = `Line ${index + 1}`;
    if (at < 0) return { error: `${where} needs the form KEY=value.` };
    const key = line.slice(0, at).trim();
    const value = line.slice(at + 1);
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key) || key.length > MAX_ENV_KEY_CHARS) {
      return { error: `${where}: “${key}” is not a valid variable name (letters, digits and _, not starting with a digit).` };
    }
    if (isReservedEnv(key)) return { error: `${where}: ${key} is managed by TermZip and cannot be set.` };
    if (new TextEncoder().encode(value).length > MAX_ENV_VALUE_BYTES) {
      return { error: `${where}: the value of ${key} is too long.` };
    }
    env[key] = value;
    if (Object.keys(env).length > MAX_ENV_VARS) return { error: `At most ${MAX_ENV_VARS} variables per profile.` };
  }
  return { env };
}

export function formatEnv(env: Record<string, string>): string {
  return Object.entries(env)
    .map(([key, value]) => `${key}=${value}`)
    .join("\n");
}

export type FontSizeResult = { size: number | null } | { error: string };

/** The font size field of the profile form: empty means "same as the app", otherwise a whole number in range. */
export function parseFontSize(text: string): FontSizeResult {
  const trimmed = text.trim();
  if (!trimmed) return { size: null };
  const size = Number(trimmed);
  const { min, max } = LIMITS.fontSize;
  if (!/^\d+$/.test(trimmed) || size < min || size > max) {
    return { error: `The font size must be a whole number from ${min} to ${max}, or empty.` };
  }
  return { size };
}

/** One line for the list: where the profile starts, how it looks, and how many variables it sets. */
export function describeProfile(profile: Profile, themeNames: ReadonlyMap<string, string> = new Map()): string {
  const parts: string[] = [];
  if (profile.shell) parts.push(profile.shell);
  if (profile.workingDirectory) parts.push(profile.workingDirectory);
  if (profile.theme) parts.push(themeNames.get(profile.theme) ?? `${profile.theme} (missing)`);
  if (profile.fontSize) parts.push(`${profile.fontSize}px`);
  if (profile.fontFamily) parts.push(profile.fontFamily);
  if (profile.cursorStyle) parts.push(`${profile.cursorStyle} cursor`);
  const variables = Object.keys(profile.env).length;
  if (variables > 0) parts.push(`${variables} variable${variables === 1 ? "" : "s"}`);
  return parts.length > 0 ? parts.join(" · ") : "Same as the built-in profile";
}
