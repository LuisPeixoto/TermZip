/**
 * What one terminal pane is drawn with: the app's settings, unless the pane's profile sets its own
 * theme, font or cursor. The window around the panes always follows the app's theme.
 */
import type { CursorStyle, Profile, Settings, Theme, ThemeEntry } from "./api";
import { BUILT_IN_ID } from "./profiles";
import { withAlpha } from "./theme";

export interface PaneLook {
  /** The colors of the terminal. */
  theme: Theme;
  /** The profile chose a theme of its own, so the pane has to paint its own background. */
  ownTheme: boolean;
  fontSize: number;
  fontFamily: string | null;
  cursorStyle: CursorStyle;
}

/**
 * The profile a pane starts with, the way the backend resolves it: the built-in profile when asked for
 * by name, otherwise the one asked for, otherwise the default profile (if any).
 */
export function profileFor(
  settings: Pick<Settings, "profiles" | "defaultProfile">,
  profileId: string | null,
): Profile | null {
  if (profileId === BUILT_IN_ID) return null;
  const find = (id: string | null) => settings.profiles.find((profile) => profile.id === id) ?? null;
  return find(profileId) ?? find(settings.defaultProfile);
}

export function resolvePaneLook(input: {
  settings: Pick<Settings, "profiles" | "defaultProfile" | "fontSize" | "fontFamily" | "cursorStyle">;
  entries: readonly ThemeEntry[];
  /** The theme the window uses, which a pane without a theme of its own shares. */
  appTheme: Theme | null;
  profileId: string | null;
}): PaneLook | null {
  const { settings, entries, appTheme } = input;
  const profile = profileFor(settings, input.profileId);
  const own = profile?.theme ? entries.find((entry) => entry.theme.id === profile.theme)?.theme : undefined;
  const theme = own ?? appTheme;
  if (!theme) return null;
  return {
    theme,
    // A profile whose theme no longer exists simply follows the app, like any other pane.
    ownTheme: own !== undefined,
    fontSize: profile?.fontSize ?? settings.fontSize,
    fontFamily: profile?.fontFamily ?? settings.fontFamily,
    cursorStyle: profile?.cursorStyle ?? settings.cursorStyle,
  };
}

/**
 * The background a pane with a theme of its own paints behind its terminal, with the window's opacity
 * so a translucent window stays translucent; `undefined` for a pane that shares the window's surface.
 */
export function paneSurface(look: PaneLook, opacity: number): string | undefined {
  if (!look.ownTheme) return undefined;
  const { background } = look.theme.colors;
  return opacity >= 1 ? background : withAlpha(background, Number(Math.max(0, opacity).toFixed(2)));
}
