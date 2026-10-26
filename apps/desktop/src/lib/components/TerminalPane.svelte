<script lang="ts">
  import { onMount } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import "@xterm/xterm/css/xterm.css";
  import {
    api,
    base64ToBytes,
    onPtyData,
    onPtyExit,
    type PtyData,
    type PtyExit,
  } from "../api";
  import { xtermTheme } from "../terminal/xtermTheme";
  import { app } from "../stores/app.svelte";

  let container: HTMLDivElement | undefined = $state();
  let terminal: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  let webgl: WebglAddon | undefined;
  let sessionInfo: { id: number; shell: string; integrated: boolean } | null = null;
  let disposed = false;

  $effect(() => {
    if (terminal && sessionInfo) {
      terminal.options.fontSize = app.settings.fontSize;
      terminal.options.fontFamily = app.settings.fontFamily ?? undefined;
      fit();
    }
  });

  $effect(() => {
    if (terminal) {
      // Re-read the CSS custom properties after a theme switch.
      requestAnimationFrame(() => {
        terminal!.options.theme = xtermTheme();
      });
    }
  });

  $effect(() => {
    function onRestart() {
      restartShell();
    }
    window.addEventListener("termzip:restart", onRestart);
    return () => window.removeEventListener("termzip:restart", onRestart);
  });

  onMount(() => {
    const settings = app.settings;
    terminal = new Terminal({
      convertEol: false,
      cursorBlink: true,
      fontFamily: settings.fontFamily ?? undefined,
      fontSize: settings.fontSize,
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
    terminal.onData(onTerminalData);
    terminal.attachCustomKeyEventHandler(onCustomKey);

    const observer = new ResizeObserver(() => fit());
    observer.observe(container!);

    void startSession();

    let unlistenData: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;
    void onPtyData((event: PtyData) => {
      if (!sessionInfo || event.sessionId !== sessionInfo.id || !terminal) return;
      terminal.write(base64ToBytes(event.data));
    }).then((unlisten) => (unlistenData = unlisten));
    void onPtyExit((event: PtyExit) => {
      if (!sessionInfo || event.sessionId !== sessionInfo.id) return;
      app.markExited(event.code);
      app.toast(
        event.code === 0 ? "warn" : "err",
        `Shell exited${event.code != null ? ` with code ${event.code}` : ""}`,
      );
    }).then((unlisten) => (unlistenExit = unlisten));

    return () => {
      disposed = true;
      observer.disconnect();
      unlistenData?.();
      unlistenExit?.();
      if (sessionInfo) void api.destroySession(sessionInfo.id).catch(() => {});
      terminal?.dispose();
    };
  });

  async function startSession(): Promise<void> {
    if (!terminal || disposed) return;
    fit();
    const rows = Math.max(terminal.rows, 10);
    const columns = Math.max(terminal.cols, 20);
    try {
      const info = await api.createSession({
        rows,
        columns,
        workingDirectory: app.settings.workingDirectory,
        shell: app.settings.shell,
      });
      if (disposed) {
        void api.destroySession(info.id).catch(() => {});
        return;
      }
      sessionInfo = info;
      app.setSession(info);

      const backlog = await api.takeBacklog(info.id);
      if (backlog && terminal) terminal.write(base64ToBytes(backlog));
      terminal?.focus();
    } catch (error) {
      app.toast("err", `Could not start the shell: ${error}`);
      terminal?.writeln(
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
    app.setViewport({ rows: terminal.rows, columns: terminal.cols });
    if (sessionInfo && terminal.rows > 0 && terminal.cols > 0) {
      void api
        .resizeSession(sessionInfo.id, terminal.rows, terminal.cols)
        .catch(() => {});
    }
  }

  function onTerminalData(data: string): void {
    if (!sessionInfo) return;
    void api.writeSession(sessionInfo.id, new TextEncoder().encode(data)).catch((error) => {
      app.toast("err", `Write failed: ${error}`);
    });
  }

  function onCustomKey(event: KeyboardEvent): boolean {
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
    if (sessionInfo) {
      void api.destroySession(sessionInfo.id).catch(() => {});
    }
    sessionInfo = null;
    terminal?.clear();
    void startSession();
  }
</script>

<div bind:this={container} class="min-h-0 min-w-0 flex-1 px-3 pb-1"></div>