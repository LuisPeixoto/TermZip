import { api, type Plugin } from "../api";
import { handlePluginEvent } from "../pluginEvents";
import { app } from "./app.svelte";
import { workspace } from "./workspace.svelte";

/** The installed plugins, what they show in the status bar, and what they add to the palette. */
function createPluginStore() {
  let plugins = $state.raw<Plugin[]>([]);
  let problems = $state.raw<string[]>([]);
  let loaded = $state(false);
  // The status-bar text of each plugin, by plugin id.
  let statuses = $state.raw<Record<string, string>>({});
  let listening = false;

  function setStatus(plugin: string, text: string): void {
    const { [plugin]: _gone, ...rest } = statuses;
    statuses = text ? { ...rest, [plugin]: text } : rest;
  }

  const name = (id: string) => plugins.find((plugin) => plugin.manifest.id === id)?.manifest.name ?? id;

  async function load(): Promise<void> {
    const listing = await api.listPlugins();
    plugins = listing.plugins;
    problems = listing.problems;
    loaded = true;
    // A plugin that is off or stopped has no item in the status bar any more.
    const running = new Set(listing.plugins.filter((plugin) => plugin.running).map((plugin) => plugin.manifest.id));
    const kept = Object.entries(statuses).filter(([id]) => running.has(id));
    if (kept.length !== Object.keys(statuses).length) statuses = Object.fromEntries(kept);
  }

  /** Starts receiving what plugins answer. Once. */
  async function listen(): Promise<void> {
    if (listening) return;
    listening = true;
    await api.subscribePlugins((event) =>
      handlePluginEvent(event, {
        pane: (session) => workspace.paneBySession(session),
        toast: (kind, text) => app.toast(kind, text),
        setStatus,
        name,
        reload: () => void load().catch(() => {}),
      }),
    );
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
    get statuses() {
      return statuses;
    },
    /** The plugins that are running now. */
    get running() {
      return plugins.filter((plugin) => plugin.running);
    },
    /** Whether any running plugin has link patterns, so lines are only sent to the backend when they could match. */
    get hasLinks() {
      return plugins.some((plugin) => plugin.running && plugin.manifest.links.length > 0);
    },
    name,
    setStatus,
    load,
    listen,
  };
}

export const pluginStore = createPluginStore();
