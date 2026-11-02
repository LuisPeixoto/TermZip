import { describe, expect, it, vi } from "vitest";
import type { UpdateInfo } from "../src/lib/api";
import { checkAtStartup, failed, found, summary } from "../src/lib/updates";

const info: UpdateInfo = { version: "1.0.1", currentVersion: "1.0.0", notes: "Fixes.", date: "2026-11-10" };

describe("what a check found", () => {
  it("is an update, or that this is the newest", () => {
    expect(found(info)).toEqual({ kind: "available", info });
    expect(found(null)).toEqual({ kind: "current" });
  });

  it("says what went wrong", () => {
    expect(failed("Could not reach the update server.")).toEqual({
      kind: "error",
      message: "Could not reach the update server.",
    });
    expect(failed(new Error("boom"))).toEqual({ kind: "error", message: "Error: boom" });
  });

  it("is described in one line, with the date when there is one", () => {
    expect(summary(info)).toBe("TermZip 1.0.1 is available. You have 1.0.0; released 2026-11-10.");
    expect(summary({ ...info, date: null })).toBe("TermZip 1.0.1 is available. You have 1.0.0.");
  });
});

describe("the check at startup", () => {
  it("does nothing, and asks nothing, unless it was turned on", async () => {
    const check = vi.fn();
    const notify = vi.fn();

    await checkAtStartup({ enabled: false, check, notify });

    expect(check).not.toHaveBeenCalled();
    expect(notify).not.toHaveBeenCalled();
  });

  it("says something only when there is an update", async () => {
    const notify = vi.fn();

    await checkAtStartup({ enabled: true, check: async () => null, notify });
    expect(notify).not.toHaveBeenCalled();

    await checkAtStartup({ enabled: true, check: async () => info, notify });
    expect(notify).toHaveBeenCalledTimes(1);
    expect(notify.mock.calls[0][0]).toContain("TermZip 1.0.1 is available");
    expect(notify.mock.calls[0][0]).toContain("Preferences → General");
  });

  it("stays silent when there is no connection", async () => {
    const notify = vi.fn();

    await expect(
      checkAtStartup({ enabled: true, check: async () => Promise.reject("no network"), notify }),
    ).resolves.toBeUndefined();

    expect(notify).not.toHaveBeenCalled();
  });
});
