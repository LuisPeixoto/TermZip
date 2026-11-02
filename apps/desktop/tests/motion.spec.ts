import { afterEach, describe, expect, it, vi } from "vitest";
import { mayBlink } from "../src/lib/stores/motion.svelte";

describe("the blinking cursor", () => {
  it("blinks only when it is wanted and the system does not ask for less motion", () => {
    expect(mayBlink(true, false)).toBe(true);
    expect(mayBlink(true, true)).toBe(false);
    expect(mayBlink(false, false)).toBe(false);
    expect(mayBlink(false, true)).toBe(false);
  });
});

describe("the preference of the system", () => {
  const original = window.matchMedia;
  afterEach(() => {
    window.matchMedia = original;
    vi.resetModules();
  });

  function stubMatchMedia(initial: boolean) {
    let listener: ((event: { matches: boolean }) => void) | undefined;
    window.matchMedia = ((query: string) => ({
      matches: query.includes("reduce") ? initial : false,
      addEventListener: (_: string, callback: (event: { matches: boolean }) => void) => (listener = callback),
    })) as unknown as typeof window.matchMedia;
    return (matches: boolean) => listener?.({ matches });
  }

  it("is read when the app starts and followed when it changes", async () => {
    const change = stubMatchMedia(true);
    const { reducedMotion } = await import("../src/lib/stores/motion.svelte");
    expect(reducedMotion.value).toBe(true);

    change(false);
    expect(reducedMotion.value).toBe(false);
  });

  it("is off when the system says nothing", async () => {
    stubMatchMedia(false);
    const { reducedMotion } = await import("../src/lib/stores/motion.svelte");

    expect(reducedMotion.value).toBe(false);
  });
});
