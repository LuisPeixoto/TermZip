/** Output the terminal has to catch up with before the backend pauses (see `flow.rs`). */
export const ACK_BATCH = 64 * 1024;

/**
 * Counts the bytes xterm has finished parsing and reports them in batches, so a flood costs one
 * message per 64 KiB rather than one per chunk. Nothing is reported until `ready()` says the
 * session id is known; the count is kept and goes out with the next flush.
 */
export class AckBatcher {
  private processed = 0;

  constructor(
    private readonly batch: number,
    private readonly send: (bytes: number) => void,
    private readonly ready: () => boolean,
  ) {}

  add(bytes: number): void {
    this.processed += bytes;
    this.flush();
  }

  flush(): void {
    if (this.processed < this.batch || !this.ready()) return;
    const bytes = this.processed;
    this.processed = 0;
    this.send(bytes);
  }

  /** A new shell starts counting from zero. */
  reset(): void {
    this.processed = 0;
  }
}

/** The most input held while the session id is not known yet. */
export const MAX_QUEUED_INPUT = 64 * 1024;

/**
 * What the terminal says to the shell before the shell's session id is known: the answers to the questions a shell
 * (or ConPTY, which always asks where the cursor is before it runs anything) puts to the terminal in its very first
 * output, and keys pressed in that moment. Output arrives before `createSession` has answered, so without this the
 * answer was dropped and, on Windows, the shell waited for it for ever: a blank terminal.
 */
export class InputQueue {
  private parts: string[] = [];
  private size = 0;

  /** Holds `data` for later; what would pass the limit is dropped. */
  push(data: string): void {
    if (this.size + data.length > MAX_QUEUED_INPUT) return;
    this.parts.push(data);
    this.size += data.length;
  }

  /** Everything held, in order. The queue is empty afterwards. */
  drain(): string[] {
    const parts = this.parts;
    this.parts = [];
    this.size = 0;
    return parts;
  }

  /** A new shell starts with nothing held for it. */
  clear(): void {
    this.drain();
  }
}
