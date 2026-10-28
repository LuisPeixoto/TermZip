<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import "@xterm/xterm/css/xterm.css";
  import { api, base64ToBytes, type SessionEvent } from "../api";
  import { xtermTheme } from "../terminal/xtermTheme";
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import type { Pane } from "../stores/pane.svelte";
  import { matchShortcut } from "../shortcuts";
  import { ACK_BATCH, AckBatcher } from "../flow";

  let { pane }: { pane: Pane } = $props();

  const DEFAULT_FONT_FAMILY =
    '"JetBrains Mono", "SF Mono", "Cascadia Code", Menlo, Consolas, "Liberation Mono", monospace';

  function fontFamily(): string {
    return app.settings.fontFamily ?? DEFAULT_FONT_FAMILY;
  }

  let container: HTMLDivElement | undefined = $state();
  // Reactive on purpose: the effects below must run again once the terminal exists.
  let terminal: Terminal | undefined = $state.raw();
  let fitAddon: FitAddon | undefined;
  let webgl: WebglAddon | undefined;
  let appliedTheme = app.settings.theme;
  let disposed = false;
  // Each shell gets a number; events of an older shell (a restart is in flight) are ignored.
  let generation = 0;
  // Tells the backend how much output has been drawn, so a flooding shell is paused, not the webview.
  const acks = new AckBatcher(
    ACK_BATCH,
    (bytes) => {
      if (pane.session) void api.ackSession(pane.session.id, bytes).catch(() => {});
    },
    () => pane.session !== null,
  );

  // Preferences apply to the live terminal; the buffer and the shell are left alone.
  $effect(() => {
    const { fontSize, scrollbackLines } = app.settings;
    const family = fontFamily();
    if (!terminal) return;
    terminal.options.fontSize = fontSize;
    terminal.options.fontFamily = family;
    terminal.options.scrollback = scrollbackLines;
    untrack(() => fit());
    requestAnimationFrame(() => fit());
  });

  $effect(() => {
    const theme = app.settings.theme;
    if (!terminal || theme === appliedTheme) return;
    appliedTheme = theme;
    // The theme tokens are written to the document by the store; read them on the next frame.
    requestAnimationFrame(() => applyTerminalTheme());
  });

  function applyTerminalTheme(): void {
    if (!terminal) return;
    terminal.options.theme = xtermTheme();
    // The WebGL renderer keeps its clear color, so rebuild it on theme changes
    // while leaving the terminal buffer untouched.
    if (webgl) {
      try {
        webgl.dispose();
      } catch {
        /* already disposed */
      }
      webgl = undefined;
      try {
        const addon = new WebglAddon();
        terminal.loadAddon(addon);
        addon.onContextLoss(() => {
          addon.dispose();
          if (webgl === addon) webgl = undefined;
        });
        webgl = addon;
      } catch {
        webgl = undefined;
      }
    }
    terminal.refresh(0, Math.max(0, terminal.rows - 1));
  }

  $effect(() => {
    // These reach every pane; only the one that has the keyboard answers.
    const isActive = () => workspace.activePane.id === pane.id;
    function onRestart() {
      if (isActive()) restartShell();
    }
    function onFocusRequest() {
      if (isActive()) terminal?.focus();
    }
    // Coming back to the app must not leave the keyboard on nothing: type first, click never.
    function onWindowFocus() {
      const active = document.activeElement;
      if (!isActive() || app.settingsOpen || (active && active !== document.body)) return;
      terminal?.focus();
    }
    window.addEventListener("termzip:restart", onRestart);
    window.addEventListener("termzip:focus-terminal", onFocusRequest);
    window.addEventListener("focus", onWindowFocus);
    return () => {
      window.removeEventListener("termzip:restart", onRestart);
      window.removeEventListener("termzip:focus-terminal", onFocusRequest);
      window.removeEventListener("focus", onWindowFocus);
    };
  });

  onMount(() => {
    const settings = app.settings;
    terminal = new Terminal({
      convertEol: false,
      cursorBlink: true,
      fontFamily: fontFamily(),
      fontSize: settings.fontSize,
      fontWeight: 400,
      fontWeightBold: 600,
      lineHeight: 1.3,
      letterSpacing: 0,
      scrollback: settings.scrollbackLines,
      theme: xtermTheme(),
    });
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container!);

    try {
      webgl = new WebglAddon();
      terminal.loadAddon(webgl);
      webgl.onContextLoss(() => {
        webgl?.dispose();
        webgl = undefined;
      });
    } catch {
      webgl = undefined; // keep the canvas renderer
    }

    fit();
    pane.controller = { restart: restartShell, focus: () => terminal?.focus() };
    terminal.onData(onTerminalData);
    terminal.attachCustomKeyEventHandler(onCustomKey);
    terminal.onTitleChange((title) => (pane.title = title.trim() || null));

    const observer = new ResizeObserver(() => fit());
    observer.observe(container!);
    const onWindowResize = () => fit();
    window.addEventListener("resize", onWindowResize);
    // WebFonts and the Tauri window can settle after the first frame.
    void document.fonts.ready.then(() => fit());
    const fitTimer = window.setTimeout(() => fit(), 250);

    void startSession();

    return () => {
      disposed = true;
      observer.disconnect();
      window.removeEventListener("resize", onWindowResize);
      window.clearTimeout(fitTimer);
      pane.controller = null;
      if (pane.session) void api.destroySession(pane.session.id).catch(() => {});
      terminal?.dispose();
    };
  });

  function onSessionEvent(owner: number, event: SessionEvent): void {
    if (disposed || owner !== generation || !terminal) return;
    switch (event.type) {
      case "data": {
        const bytes = base64ToBytes(event.data);
        terminal.write(bytes, () => {
          if (owner === generation) acks.add(bytes.length);
        });
        break;
      }
      case "block":
        pane.addBlock(event.block);
        break;
      case "cwd":
        pane.cwd = event.path;
        break;
      case "exit":
        pane.markExited(event.code);
        app.toast(
          event.code === 0 ? "warn" : "err",
          `Shell exited${event.code != null ? ` with code ${event.code}` : ""}`,
        );
        break;
    }
  }

  async function startSession(): Promise<void> {
    if (!terminal || disposed) return;
    const owner = ++generation;
    acks.reset();
    pane.begin();
    fit();
    const rows = Math.max(terminal.rows, 10);
    const columns = Math.max(terminal.cols, 20);

    try {
      const info = await api.createSession(
        {
          rows,
          columns,
          profileId: pane.profileId,
          restoreDirectory: pane.restoreDirectory,
        },
        (event) => onSessionEvent(owner, event),
      );
      if (disposed || owner !== generation) {
        // A newer shell (or the window closing) replaced this one while it started.
        void api.destroySession(info.id).catch(() => {});
        return;
      }
      pane.restoreDirectory = null; // only the first start goes back to where the pane was
      pane.setSession(info);
      acks.flush(); // output that arrived before the id was known
      if (!app.settingsOpen) terminal.focus();
    } catch (error) {
      app.toast("err", `Could not start the shell: ${error}`);
      terminal.writeln(
        "\x1b[31mCould not start the shell. Check the shell path in Preferences.\x1b[0m",
      );
    }
  }

  function fit(): void {
    if (!terminal || !fitAddon || !container) return;
    try {
      fitAddon.fit();
    } catch {
      return;
    }
    pane.viewport = { rows: terminal.rows, columns: terminal.cols };
    if (pane.session && terminal.rows > 0 && terminal.cols > 0) {
      void api
        .resizeSession(pane.session.id, terminal.rows, terminal.cols)
        .catch(() => {});
    }
  }

  function onTerminalData(data: string): void {
    if (!pane.session) return;
    void api.writeSession(pane.session.id, new TextEncoder().encode(data)).catch((error) => {
      app.toast("err", `Write failed: ${error}`);
    });
  }

  function onCustomKey(event: KeyboardEvent): boolean {
    // Application shortcuts must not reach the shell (Ctrl+Shift+R would arrive as Ctrl+R).
    if (matchShortcut(event, app.platform)) return false;
    const meta = event.metaKey || (event.ctrlKey && event.shiftKey);
    if (!meta) return true;

    if (event.type === "keydown" && (event.key === "c" || event.key === "C")) {
      const selection = terminal?.getSelection();
      if (selection) {
        void writeText(selection).catch(() => {});
        terminal?.clearSelection();
        return false;
      }
      return true; // let Ctrl+C reach the shell as SIGINT
    }
    if (event.type === "keydown" && (event.key === "v" || event.key === "V")) {
      void navigator.clipboard
        .readText()
        .then((text) => text && terminal?.paste(text))
        .catch(() => app.toast("warn", "Clipboard is unavailable in this window"));
      return false;
    }
    if (event.type === "keydown" && (event.key === "a" || event.key === "A")) {
      terminal?.selectAll();
      return false;
    }
    return true;
  }

  function restartShell(): void {
    if (pane.session) {
      void api.destroySession(pane.session.id).catch(() => {});
    }
    pane.session = null;
    // A full reset: clear() keeps the old prompt line, and the new shell then draws its own over it.
    // It also drops modes the old shell left behind (alternate screen, mouse reporting).
    terminal?.reset();
    void startSession();
  }
</script>

<!-- FitAddon measures the parent's border box, so the padding lives one level up. -->
<div class="flex min-h-0 min-w-0 flex-1 flex-col bg-bg px-3 pb-2 pt-1">
  <div bind:this={container} class="termzip-terminal min-h-0 flex-1"></div>
</div>