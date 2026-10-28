/** Hands the keyboard to the terminal of the active pane, once the interface has settled. */
export function focusTerminal(): void {
  requestAnimationFrame(() => window.dispatchEvent(new CustomEvent("termzip:focus-terminal")));
}
