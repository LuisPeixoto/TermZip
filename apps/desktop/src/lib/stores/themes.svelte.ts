import { api, type ThemeEntry } from "../api";

/** The gallery: built-in themes first, then the ones the user added. */
function createThemeStore() {
  let entries = $state.raw<ThemeEntry[]>([]);
  let problems = $state.raw<string[]>([]);

  async function load(): Promise<void> {
    const list = await api.listThemes();
    entries = list.themes;
    problems = list.problems;
  }

  return {
    get entries() {
      return entries;
    },
    get problems() {
      return problems;
    },
    load,
  };
}

export const themes = createThemeStore();
