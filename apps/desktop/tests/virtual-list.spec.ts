import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import VirtualList from "../src/lib/components/VirtualList.svelte";

// happy-dom has no layout: every element measures 0, so the viewport is empty and only the
// overscan around the top is rendered, which is exactly what is being checked.
class FakeResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", FakeResizeObserver);
  target = document.createElement("div");
  document.body.append(target);
});

afterEach(() => {
  if (component) unmount(component);
  component = undefined;
  target.remove();
  vi.unstubAllGlobals();
});

const row = createRawSnippet<[number]>((item) => ({
  render: () => `<p class="row">item ${item()}</p>`,
}));

function render(count: number) {
  component = mount(VirtualList<number>, {
    target,
    props: { items: Array.from({ length: count }, (_, index) => index), keyOf: (n: number) => n, row },
  });
  flushSync();
}

describe("VirtualList", () => {
  it("renders a window of a long list, not all of it", () => {
    render(5000);
    const rendered = target.querySelectorAll(".row").length;
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(10);
    expect(target.textContent).toContain("item 0");
    expect(target.textContent).not.toContain("item 4999");
  });

  it("reserves the room of the items it leaves out, so the scrollbar is the right size", () => {
    render(5000);
    const spacer = target.querySelector<HTMLElement>("[style*='padding-bottom']");
    const bottom = Number.parseFloat(spacer?.style.paddingBottom ?? "0");
    // 5000 items at the 150px estimate, minus the few that are rendered.
    expect(bottom).toBeGreaterThan(5000 * 150 - 10 * 150);
  });

  it("renders a short list completely", () => {
    render(2);
    expect(target.querySelectorAll(".row")).toHaveLength(2);
  });

  it("renders nothing for an empty list", () => {
    render(0);
    expect(target.querySelectorAll(".row")).toHaveLength(0);
  });
});
