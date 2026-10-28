import type { Settings } from "./api";

/** What the app shows before the saved settings arrive; the same values the backend starts from. */
export function defaultSettings(): Settings {
  return {
    theme: "dark",
    darkTheme: "termzip-dark",
    lightTheme: "termzip-light",
    cursorStyle: "block",
    cursorBlink: true,
    lineHeight: 1.3,
    letterSpacing: 0,
    backgroundOpacity: 1,
    backgroundBlur: true,
    fontSize: 14,
    fontFamily: null,
    scrollbackLines: 10_000,
    shell: null,
    workingDirectory: null,
    restoreSession: true,
    defaultProfile: null,
    keybindings: {},
    profiles: [],
  };
}

/** The ranges the backend clamps to; the sliders use the same ones. */
export const LIMITS = {
  fontSize: { min: 10, max: 28 },
  lineHeight: { min: 1, max: 2 },
  letterSpacing: { min: 0, max: 4 },
  backgroundOpacity: { min: 0.5, max: 1 },
} as const;
