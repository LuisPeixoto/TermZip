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
