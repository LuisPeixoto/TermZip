export interface Toast {
  id: number;
  kind: "info" | "ok" | "warn" | "err";
  message: string;
}

const VISIBLE = 4;
const LIFETIME_MS = 2600;

let sequence = 0;
let items = $state<Toast[]>([]);

/** The toasts on screen, newest last. */
export function currentToasts(): Toast[] {
  return items;
}

export function toast(kind: Toast["kind"], message: string): void {
  sequence += 1;
  const entry: Toast = { id: sequence, kind, message };
  items = [...items.slice(-(VISIBLE - 1)), entry];
  setTimeout(() => {
    items = items.filter((candidate) => candidate.id !== entry.id);
  }, LIFETIME_MS);
}
