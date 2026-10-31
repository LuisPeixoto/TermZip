/** What the window does with what plugins tell it. Pure, with the window's parts handed in, so it is tested alone. */
import type { PluginEvent, Tone } from "./api";

/** A badge a plugin put on a block. */
export interface PluginLabel {
  plugin: string;
  text: string;
  tone: Tone;
}

/** Labels of one terminal, by the index of the block they are on. */
export type Labels = Record<number, PluginLabel[]>;

/** The most badges one block carries, so a chatty plugin cannot fill a card. */
export const MAX_LABELS_PER_BLOCK = 4;

/** Adds a label to a block. A plugin has one label per block: a new one replaces its old one. */
export function addLabel(labels: Labels, blockIndex: number, label: PluginLabel): Labels {
  const current = (labels[blockIndex] ?? []).filter((known) => known.plugin !== label.plugin);
  return { ...labels, [blockIndex]: [...current, label].slice(-MAX_LABELS_PER_BLOCK) };
}

/** Drops the labels of blocks that are no longer listed. */
export function pruneLabels(labels: Labels, keep: readonly number[]): Labels {
  const kept = new Set(keep);
  const entries = Object.entries(labels).filter(([index]) => kept.has(Number(index)));
  return entries.length === Object.keys(labels).length ? labels : Object.fromEntries(entries);
}

/** The parts of the window a plugin event touches. */
export interface PluginEventTargets {
  /** The terminal with this session id, if it is still open. */
  pane(session: number): { addLabel(blockIndex: number, label: PluginLabel): void; paste(text: string): void } | undefined;
  toast(kind: Tone, text: string): void;
  setStatus(plugin: string, text: string): void;
  /** The name to show for a plugin id. */
  name(plugin: string): string;
  /** A plugin stopped or crashed: its row in the list is out of date. */
  reload(): void;
}

export function handlePluginEvent(event: PluginEvent, targets: PluginEventTargets): void {
  switch (event.type) {
    case "label":
      targets.pane(event.session)?.addLabel(event.blockIndex, {
        plugin: event.plugin,
        text: event.text,
        tone: event.tone,
      });
      break;
    case "notify":
      targets.toast(event.tone, `${targets.name(event.plugin)}: ${event.text}`);
      break;
    case "status":
      targets.setStatus(event.plugin, event.text);
      break;
    case "type":
      // A paste, not keystrokes: the terminal decides how (bracketed paste when the program asked for it).
      targets.pane(event.session)?.paste(event.text);
      break;
    case "fault":
      targets.toast(
        event.stopped ? "err" : "warn",
        event.stopped
          ? `${targets.name(event.plugin)} was stopped: ${event.reason}.`
          : `${targets.name(event.plugin)}: ${event.reason}.`,
      );
      if (event.stopped) {
        targets.setStatus(event.plugin, "");
        targets.reload();
      }
      break;
  }
}
