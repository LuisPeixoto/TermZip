/** What the interface says about a plugin, as pure functions. */
import type { CatalogItem, Plugin, PluginManifest, PluginPermission } from "./api";

export interface PermissionChip {
  permission: PluginPermission;
  label: string;
  /** Worth a second look: it reads output, types into the terminal or opens addresses. */
  sensitive: boolean;
}

const CHIPS: Record<PluginPermission, { label: string; sensitive: boolean }> = {
  blocks: { label: "Sees your commands", sensitive: false },
  output: { label: "Sees what they print", sensitive: true },
  type: { label: "Types into the terminal", sensitive: true },
  notify: { label: "Shows messages", sensitive: false },
  open: { label: "Opens web addresses", sensitive: true },
};

export function permissionChips(manifest: PluginManifest): PermissionChip[] {
  return manifest.permissions.map((permission) => ({ permission, ...CHIPS[permission] }));
}

const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

/** What a plugin adds, in one line: its palette commands and its link patterns. */
export function describeAdds(manifest: PluginManifest): string {
  const parts: string[] = [];
  if (manifest.commands.length > 0) parts.push(plural(manifest.commands.length, "command", "commands"));
  if (manifest.links.length > 0) parts.push(plural(manifest.links.length, "link pattern", "link patterns"));
  if (manifest.permissions.includes("blocks")) parts.push("reacts to commands");
  return parts.length > 0 ? parts.join(" · ") : "adds nothing";
}

export function describeSource(plugin: Plugin): { label: string; verified: boolean } {
  return plugin.source === "catalog"
    ? { label: "From the catalog, checked against its signature", verified: true }
    : { label: "From a file: not verified by anyone", verified: false };
}

export type PluginState = "running" | "stopped" | "changed" | "off";

/** `stopped`: approved and loaded once, but it crashed three times in a row and was stopped. */
export function stateOf(plugin: Plugin): PluginState {
  if (plugin.changed) return "changed";
  if (!plugin.enabled) return "off";
  return plugin.running ? "running" : "stopped";
}

export function describeState(state: PluginState): string {
  switch (state) {
    case "running":
      return "On";
    case "stopped":
      return "Stopped: it crashed three times in a row. Turn it off and on to try again.";
    case "changed":
      return "Off: the file changed after you turned it on.";
    case "off":
      return "Off";
  }
}

/** The sentence that precedes the list of permissions in the confirmation. */
export function describeAsk(plugin: Plugin): string {
  return plugin.manifest.permissions.length > 0
    ? `“${plugin.manifest.name}” asks to:`
    : `“${plugin.manifest.name}” asks for no permission: it can only show labels and a status, add commands and make links.`;
}

/** What the button of a catalog entry says and whether it can be pressed. */
export function catalogAction(item: CatalogItem): { label: string; enabled: boolean } {
  if (item.installedVersion === null) return { label: "Install", enabled: true };
  if (item.updateAvailable) return { label: `Update to ${item.version}`, enabled: true };
  return { label: item.installedVersion === item.version ? "Installed" : "Installed (newer)", enabled: false };
}

export function permissionChipsOf(permissions: readonly PluginPermission[]): PermissionChip[] {
  return permissions.map((permission) => ({ permission, ...CHIPS[permission] }));
}

/** `1.2 MB`, `640 KB`, `90 B`. */
export function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}
