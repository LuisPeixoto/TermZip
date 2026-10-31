import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(),
}));

import type { Block, PluginEvent } from "../src/lib/api";
import {
  MAX_LABELS_PER_BLOCK,
  addLabel,
  handlePluginEvent,
  pruneLabels,
  type PluginEventTargets,
  type PluginLabel,
} from "../src/lib/pluginEvents";
import { Pane } from "../src/lib/stores/pane.svelte";

const label = (plugin: string, text: string, tone: PluginLabel["tone"] = "ok"): PluginLabel => ({ plugin, text, tone });

describe("labels on blocks", () => {
  it("add up, one per plugin, newest last", () => {
    let labels = addLabel({}, 4, label("a", "slow"));
    labels = addLabel(labels, 4, label("b", "ok"));
    expect(labels[4].map((l) => l.text)).toEqual(["slow", "ok"]);

    // A plugin that labels the same block again replaces its own label.
    labels = addLabel(labels, 4, label("a", "very slow"));
    expect(labels[4].map((l) => l.text)).toEqual(["ok", "very slow"]);
    expect(labels).not.toBe(addLabel(labels, 4, label("a", "very slow")));
  });

  it("are capped per block so a chatty plugin cannot fill a card", () => {
    let labels = {};
    for (let n = 0; n < 10; n += 1) labels = addLabel(labels, 1, label(`p${n}`, `l${n}`));
    expect((labels as Record<number, PluginLabel[]>)[1]).toHaveLength(MAX_LABELS_PER_BLOCK);
    expect((labels as Record<number, PluginLabel[]>)[1].at(-1)?.text).toBe("l9");
  });

  it("go with their block", () => {
    const labels = addLabel(addLabel({}, 1, label("a", "x")), 2, label("a", "y"));

    expect(Object.keys(pruneLabels(labels, [2, 3]))).toEqual(["2"]);
    // Nothing to drop returns the same object, so nothing re-renders.
    expect(pruneLabels(labels, [1, 2])).toBe(labels);
    expect(pruneLabels({}, [1])).toEqual({});
  });
});

function targets() {
  const calls: string[] = [];
  const pane = {
    addLabel: vi.fn((index: number, l: PluginLabel) => calls.push(`label ${index} ${l.text}`)),
    paste: vi.fn((text: string) => calls.push(`paste ${text}`)),
  };
  const target: PluginEventTargets = {
    pane: (session) => (session === 7 ? pane : undefined),
    toast: vi.fn((kind, text) => calls.push(`toast ${kind} ${text}`)),
    setStatus: vi.fn((plugin, text) => calls.push(`status ${plugin} ${text}`)),
    name: (plugin) => (plugin === "demo" ? "Demo plugin" : plugin),
    reload: vi.fn(() => calls.push("reload")),
  };
  return { target, calls, pane };
}

describe("what the window does with an event", () => {
  const send = (event: PluginEvent) => {
    const { target, calls } = targets();
    handlePluginEvent(event, target);
    return calls;
  };

  it("labels a block of the terminal the command ran in", () => {
    expect(send({ type: "label", plugin: "demo", session: 7, blockIndex: 3, text: "slow", tone: "warn" })).toEqual([
      "label 3 slow",
    ]);
    // A terminal that was closed meanwhile is simply not there.
    expect(send({ type: "label", plugin: "demo", session: 99, blockIndex: 3, text: "slow", tone: "warn" })).toEqual([]);
  });

  it("shows a message with the plugin's name, as an info, ok, warning or error", () => {
    expect(send({ type: "notify", plugin: "demo", text: "Build done", tone: "ok" })).toEqual([
      "toast ok Demo plugin: Build done",
    ]);
    expect(send({ type: "notify", plugin: "x", text: "Hm", tone: "warn" })).toEqual(["toast warn x: Hm"]);
  });

  it("sets and clears a status item", () => {
    expect(send({ type: "status", plugin: "demo", text: "3 failed" })).toEqual(["status demo 3 failed"]);
    expect(send({ type: "status", plugin: "demo", text: "" })).toEqual(["status demo "]);
  });

  it("pastes typed text into the terminal the command was run from, never into another", () => {
    expect(send({ type: "type", plugin: "demo", session: 7, text: "git status" })).toEqual(["paste git status"]);
    expect(send({ type: "type", plugin: "demo", session: 1, text: "git status" })).toEqual([]);
  });

  it("warns about a fault, and when the plugin was stopped clears its status and reloads the list", () => {
    expect(send({ type: "fault", plugin: "demo", reason: "it crashed", stopped: false })).toEqual([
      "toast warn Demo plugin: it crashed.",
    ]);
    expect(send({ type: "fault", plugin: "demo", reason: "it crashed", stopped: true })).toEqual([
      "toast err Demo plugin was stopped: it crashed.",
      "status demo ",
      "reload",
    ]);
  });
});

describe("a terminal's labels", () => {
  beforeEach(() => vi.clearAllMocks());

  const block = (index: number): Block => ({
    index,
    command: "x",
    cwd: null,
    exitCode: 0,
    startedAtMs: null,
    durationMs: null,
    outputChars: 0,
    truncated: false,
    output: null,
  });

  it("belong to its blocks: they go when the block does and when a new shell starts", () => {
    const pane = new Pane("a");
    pane.addBlock(block(0));
    pane.addBlock(block(1));
    pane.addLabel(0, label("p", "one"));
    pane.addLabel(1, label("p", "two"));
    expect(Object.keys(pane.labels)).toEqual(["0", "1"]);

    pane.setBlocks([block(1)]);
    expect(Object.keys(pane.labels)).toEqual(["1"]);

    pane.begin();
    expect(pane.labels).toEqual({});
  });

  it("keeps a label that arrives just before its block does", () => {
    const pane = new Pane("a");
    pane.addLabel(5, label("p", "early"));
    pane.addBlock(block(5));

    expect(pane.labels[5]?.[0].text).toBe("early");
  });

  it("pastes through the terminal, and quietly does nothing without one", () => {
    const pane = new Pane("a");
    pane.paste("ignored");

    const paste = vi.fn();
    pane.controller = { restart: vi.fn(), focus: vi.fn(), clearSelection: vi.fn(), paste };
    pane.paste("git status");
    expect(paste).toHaveBeenCalledWith("git status");
  });
});
