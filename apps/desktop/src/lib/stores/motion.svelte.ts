/**
 * Whether the system asks for less motion (`prefers-reduced-motion`). CSS answers it for the interface; the terminal's
 * blinking cursor is drawn by xterm.js, so it has to be told.
 */
class ReducedMotion {
  value = $state(false);

  constructor() {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.value = query.matches;
    query.addEventListener("change", (event) => (this.value = event.matches));
  }
}

export const reducedMotion = new ReducedMotion();

/** Whether the cursor may blink: the person wants it to, and the system does not ask for less motion. */
export const mayBlink = (wanted: boolean, reduced: boolean): boolean => wanted && !reduced;
