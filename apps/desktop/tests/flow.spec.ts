import { describe, expect, it } from "vitest";
import { AckBatcher, InputQueue, MAX_QUEUED_INPUT } from "../src/lib/flow";

function batcher(batch = 100, ready = () => true) {
  const sent: number[] = [];
  return { sent, batcher: new AckBatcher(batch, (bytes) => sent.push(bytes), ready) };
}

describe("AckBatcher", () => {
  it("stays quiet below the batch size", () => {
    const { sent, batcher: b } = batcher();
    b.add(30);
    b.add(60);
    expect(sent).toEqual([]);
  });

  it("reports everything counted once the batch is reached", () => {
    const { sent, batcher: b } = batcher();
    b.add(60);
    b.add(60);
    expect(sent).toEqual([120]);
    b.add(100);
    expect(sent).toEqual([120, 100]);
  });

  it("keeps the count until the session id is known", () => {
    let known = false;
    const { sent, batcher: b } = batcher(100, () => known);
    b.add(150);
    b.add(150);
    expect(sent).toEqual([]);
    known = true;
    b.flush();
    expect(sent).toEqual([300]);
  });

  it("starts from zero for a new shell", () => {
    const { sent, batcher: b } = batcher();
    b.add(90);
    b.reset();
    b.add(90);
    expect(sent).toEqual([]);
  });
});

describe("InputQueue", () => {
  it("holds what the terminal said before the session id was known, in order", () => {
    const queue = new InputQueue();
    // ConPTY asks where the cursor is and the terminal answers; then a key is pressed.
    queue.push("\x1b[1;1R");
    queue.push("l");
    queue.push("s");

    expect(queue.drain()).toEqual(["\x1b[1;1R", "l", "s"]);
  });

  it("is empty after it was drained, so nothing goes out twice", () => {
    const queue = new InputQueue();
    queue.push("a");
    queue.drain();

    expect(queue.drain()).toEqual([]);
  });

  it("drops what would pass the limit, and keeps what came before it", () => {
    const queue = new InputQueue();
    queue.push("first");
    queue.push("x".repeat(MAX_QUEUED_INPUT));
    queue.push("last");

    expect(queue.drain()).toEqual(["first", "last"]);
  });

  it("starts empty for a new shell", () => {
    const queue = new InputQueue();
    queue.push("left over from the shell before");
    queue.clear();

    expect(queue.drain()).toEqual([]);
  });
});
