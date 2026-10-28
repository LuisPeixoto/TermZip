import { describe, expect, it } from "vitest";
import { AckBatcher } from "../src/lib/flow";

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
