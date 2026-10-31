import { api, type Plugin } from "../api";

/** The installed plugins. Changes made in the preferences apply at once, so the list is read again after each. */
function createPluginStore() {
  let plugins = $state.raw<Plugin[]>([]);
  let problems = $state.raw<string[]>([]);
  let loaded = $state(false);

  async function load(): Promise<void> {
    const listing = await api.listPlugins();
    plugins = listing.plugins;
    problems = listing.problems;
    loaded = true;
  }

  return {
    get plugins() {
      return plugins;
    },
    get problems() {
      return problems;
    },
    get loaded() {
      return loaded;
    },
    load,
  };
}

export const pluginStore = createPluginStore();
