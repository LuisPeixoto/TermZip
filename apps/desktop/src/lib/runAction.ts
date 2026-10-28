import { focusTerminal } from "./focus";
import { tabPosition, type AppShortcut } from "./shortcuts";
import { app } from "./stores/app.svelte";
import { workspace } from "./stores/workspace.svelte";

/** Does what a shortcut stands for. The keyboard and the command palette both end up here. */
export function runAction(shortcut: AppShortcut): void {
  const position = tabPosition(shortcut);
  if (position !== null) {
    workspace.activateAt(position);
    focusTerminal();
    return;
  }
  switch (shortcut) {
    case "preferences":
      app.toggleSettings();
      break;
    case "commandPalette":
      app.togglePalette();
      break;
    case "restart":
      window.dispatchEvent(new CustomEvent("termzip:restart"));
      break;
    case "blocks":
      app.toggleBlocks();
      break;
    case "find":
      workspace.activePane.openSearch();
      break;
    case "newTab":
      if (!workspace.newTab()) app.toast("warn", "That is the most tabs a window can hold");
      focusTerminal();
      break;
    case "closePane":
      workspace.closeActivePane();
      focusTerminal();
      break;
    case "splitRight":
    case "splitDown":
      if (!workspace.splitActive(shortcut === "splitRight" ? "row" : "column")) {
        app.toast("warn", "This tab cannot hold another pane");
      }
      focusTerminal();
      break;
    case "focusLeft":
    case "focusRight":
    case "focusUp":
    case "focusDown":
      workspace.activeTab.focusSide(shortcut.slice(5).toLowerCase() as "left" | "right" | "up" | "down");
      focusTerminal();
      break;
    case "nextTab":
    case "prevTab":
      workspace.cycle(shortcut === "nextTab" ? 1 : -1);
      focusTerminal();
      break;
  }
}
