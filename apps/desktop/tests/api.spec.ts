import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => {
  class Channel<T> {
    onmessage: (message: T) => void = () => {};
  }
  return { Channel, invoke: vi.fn() };
});

import { Channel, invoke } from "@tauri-apps/api/core";
import { api, base64ToBytes, bytesToBase64, type SessionEvent } from "../src/lib/api";

describe("base64 transport helpers", () => {
  it("round-trips arbitrary binary payloads", () => {
    const bytes = Uint8Array.from([0, 1, 2, 250, 251, 255, 10, 13, 27]);
    expect(Array.from(base64ToBytes(bytesToBase64(bytes)))).toEqual(Array.from(bytes));
  });

  it("handles chunked payloads larger than 32 KiB", () => {
    const bytes = new Uint8Array(80_000);
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = index % 251;
    }
    const encoded = bytesToBase64(bytes);
    const decoded = base64ToBytes(encoded);
    expect(decoded.length).toBe(bytes.length);
    expect(decoded[79_999]).toBe(79_999 % 251);
  });

  it("encodes empty payloads", () => {
    expect(bytesToBase64(new Uint8Array(0))).toBe("");
    expect(base64ToBytes("").length).toBe(0);
  });
});

describe("session stream", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("opens the session with a channel that feeds the handler in order", async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 7, shell: "zsh", integrated: true });
    const received: SessionEvent[] = [];

    const info = await api.createSession({ rows: 24, columns: 80 }, (event) =>
      received.push(event),
    );

    expect(info.id).toBe(7);
    const [command, args] = vi.mocked(invoke).mock.calls[0] as [
      string,
      { rows: number; columns: number; onEvent: Channel<SessionEvent> },
    ];
    expect(command).toBe("create_session");
    expect(args.rows).toBe(24);
    expect(args.onEvent).toBeInstanceOf(Channel);

    args.onEvent.onmessage({ type: "data", data: "aGk=" });
    args.onEvent.onmessage({ type: "blocks", blocks: [] });
    args.onEvent.onmessage({ type: "exit", code: 3 });
    expect(received.map((event) => event.type)).toEqual(["data", "blocks", "exit"]);
  });

  it("has no backlog call: the channel carries the startup output", () => {
    expect("takeBacklog" in api).toBe(false);
  });
});


describe("themes", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("asks the backend for the gallery, and for import, export and delete by id", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await api.listThemes();
    await api.importTheme();
    await api.exportTheme("nord");
    await api.deleteTheme("mine");

    expect(vi.mocked(invoke).mock.calls).toEqual([
      ["list_themes"],
      ["import_theme"],
      ["export_theme", { id: "nord" }],
      ["delete_theme", { id: "mine" }],
    ]);
  });
});

describe("plugins", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("asks the backend for the list, and for each action by plugin id", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await api.listPlugins();
    await api.installPluginFile();
    await api.enablePlugin("demo");
    await api.disablePlugin("demo");
    await api.uninstallPlugin("demo");
    await api.runPluginCommand("demo", "go", "/work", 6);
    await api.matchPluginLinks("fixes #42");

    expect(vi.mocked(invoke).mock.calls).toEqual([
      ["list_plugins"],
      ["install_plugin_file"],
      ["enable_plugin", { id: "demo" }],
      ["disable_plugin", { id: "demo" }],
      ["uninstall_plugin", { id: "demo" }],
      ["run_plugin_command", { plugin: "demo", command: "go", cwd: "/work", session: 6 }],
      ["match_plugin_links", { line: "fixes #42" }],
    ]);

    vi.mocked(invoke).mockClear();
    await api.loadCatalog();
    await api.installFromCatalog("demo");
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ["load_catalog"],
      ["install_from_catalog", { id: "demo" }],
    ]);
  });

  it("subscribes with a channel that feeds the handler", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    const received: unknown[] = [];

    await api.subscribePlugins((event) => received.push(event));

    const [command, args] = vi.mocked(invoke).mock.calls[0] as [string, { onEvent: Channel<unknown> }];
    expect(command).toBe("subscribe_plugins");
    args.onEvent.onmessage({ type: "status", plugin: "demo", text: "1 seen" });
    expect(received).toEqual([{ type: "status", plugin: "demo", text: "1 seen" }]);
  });
});
