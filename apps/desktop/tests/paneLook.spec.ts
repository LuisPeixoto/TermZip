import { describe, expect, it } from "vitest";
import type { Profile, Settings, ThemeEntry } from "../src/lib/api";
import { paneSurface, profileFor, resolvePaneLook } from "../src/lib/paneLook";
import { builtIn, builtInThemes } from "./helpers/themes";

const entries: ThemeEntry[] = builtInThemes().map((theme) => ({ builtIn: true, theme }));
const appTheme = builtIn("termzip-dark");

function profile(id: string, extra: Partial<Profile> = {}): Profile {
  return { id, name: id, shell: null, workingDirectory: null, env: {}, ...extra };
}

const settings = (profiles: Profile[], defaultProfile: string | null = null) => ({
  profiles,
  defaultProfile,
  fontSize: 14,
  fontFamily: null as string | null,
  cursorStyle: "block" as Settings["cursorStyle"],
});

describe("which profile a pane has", () => {
  const work = profile("work");
  const other = profile("other");

  it("takes the one it started with", () => {
    expect(profileFor(settings([work, other], "other"), "work")).toBe(work);
  });

  it("falls back to the default profile, as the backend does", () => {
    expect(profileFor(settings([work, other], "other"), null)).toBe(other);
    expect(profileFor(settings([work, other], "other"), "ghost")).toBe(other);
    expect(profileFor(settings([work, other]), null)).toBeNull();
  });

  it("is the built-in profile, with no overrides, when asked for by name", () => {
    expect(profileFor(settings([work], "work"), "default")).toBeNull();
  });
});

describe("how a pane looks", () => {
  it("shares the app's look when its profile sets nothing", () => {
    const look = resolvePaneLook({ settings: settings([profile("work")]), entries, appTheme, profileId: "work" });

    expect(look).toEqual({ theme: appTheme, ownTheme: false, fontSize: 14, fontFamily: null, cursorStyle: "block" });
  });

  it("uses what its profile sets, and the app's for the rest", () => {
    const styled = profile("styled", { theme: "dracula", fontSize: 18, cursorStyle: "underline" });
    const look = resolvePaneLook({ settings: settings([styled]), entries, appTheme, profileId: "styled" });

    expect(look?.theme.id).toBe("dracula");
    expect(look?.ownTheme).toBe(true);
    expect(look?.fontSize).toBe(18);
    expect(look?.cursorStyle).toBe("underline");
    expect(look?.fontFamily).toBeNull();

    const family = profile("family", { fontFamily: "Fira Code" });
    expect(resolvePaneLook({ settings: settings([family]), entries, appTheme, profileId: "family" })?.fontFamily).toBe(
      "Fira Code",
    );
  });

  it("follows the app when the theme of its profile is gone", () => {
    const ghost = profile("ghost", { theme: "deleted" });
    const look = resolvePaneLook({ settings: settings([ghost]), entries, appTheme, profileId: "ghost" });

    expect(look?.theme).toBe(appTheme);
    expect(look?.ownTheme).toBe(false);
  });

  it("may use a theme of the other appearance for the terminal alone", () => {
    const light = profile("light", { theme: "catppuccin-latte" });
    const look = resolvePaneLook({ settings: settings([light]), entries, appTheme, profileId: "light" });

    expect(look?.theme.appearance).toBe("light");
  });

  it("has no look before the app has a theme", () => {
    expect(resolvePaneLook({ settings: settings([]), entries: [], appTheme: null, profileId: null })).toBeNull();
  });
});

describe("the background behind a pane's terminal", () => {
  const own = resolvePaneLook({
    settings: settings([profile("styled", { theme: "dracula" })]),
    entries,
    appTheme,
    profileId: "styled",
  })!;
  const shared = resolvePaneLook({ settings: settings([]), entries, appTheme, profileId: null })!;

  it("is left to the window for a pane that shares the app's theme", () => {
    expect(paneSurface(shared, 1)).toBeUndefined();
    expect(paneSurface(shared, 0.7)).toBeUndefined();
  });

  it("is the pane's own background, as translucent as the window", () => {
    expect(paneSurface(own, 1)).toBe("#282a36");
    expect(paneSurface(own, 0.7)).toBe("rgb(40 42 54 / 0.7)");
  });
});
