/**
 * Reads the themes that ship with the app straight from their files, so the tests check what is
 * shipped rather than a copy of it. The TOML reader only knows what those files use.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { Theme } from "../../src/lib/api";

// `npm test` runs from apps/desktop; the test environment has no file URL to resolve against.
const THEMES_DIR = join(process.cwd(), "../../crates/termzip-core/themes");
const APP_CSS = join(process.cwd(), "src/app.css");

type Table = Record<string, string | string[]>;

export function parseThemeToml(text: string): Theme {
  const tables: Record<string, Table> = { "": {} };
  let current = "";
  let open: { key: string; items: string[] } | null = null;

  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (open) {
      if (line.startsWith("]")) {
        tables[current][open.key] = open.items;
        open = null;
      } else {
        open.items.push(...[...line.matchAll(/"([^"]*)"/g)].map((match) => match[1]));
      }
      continue;
    }
    if (!line || line.startsWith("#")) continue;
    const section = /^\[([a-z_]+)\]$/.exec(line);
    if (section) {
      current = section[1];
      tables[current] = {};
      continue;
    }
    const pair = /^([a-z_0-9]+)\s*=\s*(.*)$/.exec(line);
    if (!pair) throw new Error(`unexpected line in a theme file: ${line}`);
    const [, key, value] = pair;
    if (value === "[") open = { key, items: [] };
    else if (value.startsWith('"')) tables[current][key] = value.slice(1, value.lastIndexOf('"'));
    else throw new Error(`unexpected value in a theme file: ${line}`);
  }

  const top = tables[""] as Record<string, string>;
  return {
    id: top.id,
    name: top.name,
    appearance: top.appearance as Theme["appearance"],
    author: top.author,
    license: top.license,
    source: top.source,
    colors: tables.colors as unknown as Theme["colors"],
    chrome: tables.chrome as Theme["chrome"],
  };
}

export function builtInThemes(): Theme[] {
  return readdirSync(THEMES_DIR)
    .filter((name) => name.endsWith(".toml"))
    .sort()
    .map((name) => parseThemeToml(readFileSync(join(THEMES_DIR, name), "utf8")));
}

export function builtIn(id: string): Theme {
  const found = builtInThemes().find((theme) => theme.id === id);
  if (!found) throw new Error(`no built-in theme ${id}`);
  return found;
}

/** The custom properties of `app.css` for the light (`:root`) and dark (`[data-theme="dark"]`) blocks. */
export function appCssTokens(): { light: Record<string, string>; dark: Record<string, string> } {
  const css = readFileSync(APP_CSS, "utf8");
  const block = (selector: string): Record<string, string> => {
    const start = css.indexOf(`${selector} {`);
    const body = css.slice(start, css.indexOf("}", start));
    return Object.fromEntries([...body.matchAll(/(--[a-z0-9-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
  };
  return { light: block(":root"), dark: block('[data-theme="dark"]') };
}
