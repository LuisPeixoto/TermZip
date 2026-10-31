<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { SearchAddon } from "@xterm/addon-search";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import SearchBar from "./SearchBar.svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import "@xterm/xterm/css/xterm.css";
  import { api, base64ToBytes, type SessionEvent, type Theme } from "../api";
  import { xtermTheme } from "../theme";
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import type { Pane } from "../stores/pane.svelte";
  import { matchShortcut } from "../shortcuts";
  import { cellsToLine, columnsOf, findPathLinks, isOpenGesture } from "../terminal/links";
  import { ACK_BATCH, AckBatcher } from "../flow";
  import { fontStack } from "../terminal/font";
  import { paneSurface, resolvePaneLook } from "../paneLook";
  import { themes } from "../stores/themes.svelte";
  import { pluginStore } from "../stores/plugins.svelte";

  let { pane }: { pane: Pane } = $props();

  // A plain click on a link only selects, so text that contains one can still be copied; opening
  // needs ⌘ (Ctrl elsewhere). The backend decides what may be opened.
  function openLink(event: MouseEvent, target: string): void {
    if (!isOpenGesture(event, app.platform)) return;
    void api.openLink(target, pane.cwd ?? pane.restoreDirectory).catch((error) => {
      app.toast("warn", String(error));
    });
  }
  const showHint = (event: MouseEvent) => (hint = { x: event.clientX, y: event.clientY });
  const hideHint = () => (hint = null);

  // What this pane is drawn with: the app's settings, unless its profile sets its own theme, font or cursor.
  const look = $derived(
    resolvePaneLook({
      settings: app.settings,
      entries: themes.entries,
      appTheme: app.theme,
      profileId: pane.profileId,
    }),
  );
  const surface = $derived(look ? paneSurface(look, app.opacity) : undefined);

  function fontFamily(): string {
    return fontStack(look?.fontFamily ?? app.settings.fontFamily);
  }

  let container: HTMLDivElement | undefined = $state();
  // Reactive on purpose: the effects below must run again once the terminal exists.
  let terminal: Terminal | undefined = $state.raw();
  let fitAddon: FitAddon | undefined;
  let search: SearchAddon | undefined = $state.raw();
  let webgl: WebglAddon | undefined;
  // What the terminal was last drawn with: the theme, and whether the window lets the desktop through.
  let appliedTheme = untrack(() => look?.theme);
  let appliedTranslucent = app.translucent;
  // Where a link is being hovered, for the "⌘+click to open" hint.
  let hint = $state.raw<{ x: number; y: number } | null>(null);
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
    const { scrollbackLines, cursorBlink, lineHeight, letterSpacing } = app.settings;
    const family = fontFamily();
    if (!terminal || !look) return;
    const { fontSize, cursorStyle } = look;
    terminal.options.fontSize = fontSize;
    terminal.options.fontFamily = family;
    terminal.options.scrollback = scrollbackLines;
    terminal.options.cursorStyle = cursorStyle;
    terminal.options.cursorBlink = cursorBlink;
    terminal.options.lineHeight = lineHeight;
    terminal.options.letterSpacing = letterSpacing;
    untrack(() => fit());
    requestAnimationFrame(() => fit());
  });

  $effect(() => {
    const theme = look?.theme;
    const translucent = app.translucent;
    if (!terminal || !theme || (theme === appliedTheme && translucent === appliedTranslucent)) return;
    appliedTheme = theme;
    appliedTranslucent = translucent;
    applyTerminalTheme(theme, translucent);
  });

  function applyTerminalTheme(theme: Theme, translucent: boolean): void {
    if (!terminal) return;
    // A clear background only works with transparency on, which the renderer decides when it is created.
    terminal.options.allowTransparency = translucent;
    terminal.options.theme = xtermTheme(theme, translucent);
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
    const initial = look;
    terminal = new Terminal({
      convertEol: false,
      cursorBlink: settings.cursorBlink,
      cursorStyle: initial?.cursorStyle ?? settings.cursorStyle,
      // A pane that is not focused keeps an outline, so it is clear where the keyboard is not.
      cursorInactiveStyle: "outline",
      fontFamily: fontFamily(),
      fontSize: initial?.fontSize ?? settings.fontSize,
      fontWeight: 400,
      fontWeightBold: 600,
      lineHeight: settings.lineHeight,
      letterSpacing: settings.letterSpacing,
      scrollback: settings.scrollbackLines,
      allowTransparency: app.translucent,
      theme: initial ? xtermTheme(initial.theme, app.translucent) : undefined,
      // The search addon marks matches with decorations, which are still a proposed API.
      allowProposedApi: true,
      // OSC 8 hyperlinks (`ls --hyperlink`, many CLIs): http and https only.
      linkHandler: { activate: (event, uri) => openLink(event, uri), hover: (event) => showHint(event), leave: hideHint },
    });
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    search = new SearchAddon();
    terminal.loadAddon(search);
    search.onDidChangeResults(({ resultIndex, resultCount }) => {
      pane.searchResults = { index: resultIndex, count: resultCount };
    });
    terminal.loadAddon(new WebLinksAddon((event, uri) => openLink(event, uri), { hover: showHint, leave: hideHint }));
    terminal.registerLinkProvider({
      provideLinks: (row, callback) => {
        const line = terminal?.buffer.active.getLine(row - 1);
        if (!line) return callback(undefined);
        const cells = Array.from({ length: line.length }, (_, column) => {
          const cell = line.getCell(column);
          return { chars: cell?.getChars() ?? "", width: cell?.getWidth() ?? 1 };
        });
        const { text, columns } = cellsToLine(cells);
        const links = findPathLinks(text).map((match) => {
          const { start, end } = columnsOf(match, columns);
          return {
            range: { start: { x: start, y: row }, end: { x: end, y: row } },
            text: match.text,
            activate: (event: MouseEvent, target: string) => openLink(event, target),
            hover: (event: MouseEvent) => showHint(event),
            leave: hideHint,
          };
        });
        callback(links.length > 0 ? links : undefined);
      },
    });
    // Link patterns of plugins: the line goes to the backend only when some running plugin has patterns.
    terminal.registerLinkProvider({
      provideLinks: (row, callback) => {
        const line = terminal?.buffer.active.getLine(row - 1);
        if (!pluginStore.hasLinks || !line) return callback(undefined);
        const cells = Array.from({ length: line.length }, (_, column) => {
          const cell = line.getCell(column);
          return { chars: cell?.getChars() ?? "", width: cell?.getWidth() ?? 1 };
        });
        const { text, columns } = cellsToLine(cells);
        void api
          .matchPluginLinks(text)
          .then((found) => {
            const links = found.map((match) => {
              const { start, end } = columnsOf({ text: match.url, start: match.start, end: match.end }, columns);
              return {
                range: { start: { x: start, y: row }, end: { x: end, y: row } },
                text: match.url,
                activate: (event: MouseEvent) => openLink(event, match.url),
                hover: (event: MouseEvent) => showHint(event),
                leave: hideHint,
              };
            });
            callback(links.length > 0 ? links : undefined);
          })
          .catch(() => callback(undefined));
      },
    });
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
    pane.controller = {
      restart: restartShell,
      focus: () => terminal?.focus(),
      clearSelection: () => terminal?.clearSelection(),
      paste: (text) => terminal?.paste(text),
    };
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
    // A tab that is not shown has no size; fitting it would shrink its shell to a sliver.
    if (container.clientWidth === 0 || container.clientHeight === 0) return;
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
    if (matchShortcut(event, app.platform, app.settings.keybindings)) return false;
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
<div
  class="relative flex min-h-0 min-w-0 flex-1 flex-col px-3 pb-2 pt-1 {surface ? '' : 'bg-window'}"
  style:background-color={surface}
>
  <div bind:this={container} class="termzip-terminal min-h-0 flex-1"></div>
  {#if pane.searchOpen && search}
    <SearchBar {pane} addon={search} />
  {/if}
  {#if hint}
    <div
      class="pointer-events-none fixed z-30 rounded-md border border-line bg-elevated px-2 py-1 text-[10.5px] text-muted shadow-pop"
      style="left:{hint.x + 12}px;top:{hint.y + 16}px"
    >
      {app.platform === "macos" ? "⌘+click" : "Ctrl+click"} to open
    </div>
  {/if}
</div>