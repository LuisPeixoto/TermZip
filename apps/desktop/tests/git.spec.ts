import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import type { GitStatus } from "../src/lib/api";
import { describeGit } from "../src/lib/git";
import { git } from "../src/lib/stores/git.svelte";

const status = (extra: Partial<GitStatus> = {}): GitStatus => ({
  branch: "main",
  detached: false,
  commit: "6004959",
  ahead: 0,
  behind: 0,
  hasUpstream: true,
  staged: 0,
  modified: 0,
  untracked: 0,
  conflicted: 0,
  truncated: false,
  ...extra,
});

describe("how the git state is shown", () => {
  it("shows a clean branch in sync with its upstream", () => {
    expect(describeGit(status())).toEqual({
      label: "main",
      sync: "",
      dirty: false,
      conflicted: false,
      title: "main · in sync with its upstream · no changes",
    });
  });

  it("shows how far the branch is from its upstream", () => {
    const view = describeGit(status({ ahead: 2, behind: 1 }));
    expect(view.sync).toBe("↑2 ↓1");
    expect(view.title).toContain("2 commits ahead, 1 commit behind");
    expect(describeGit(status({ ahead: 1 })).sync).toBe("↑1");
    expect(describeGit(status({ behind: 3 })).sync).toBe("↓3");
  });

  it("says when there is no upstream to compare with", () => {
    const view = describeGit(status({ hasUpstream: false }));
    expect(view.sync).toBe("");
    expect(view.title).toContain("no upstream");
  });

  it("counts what changed, by kind, and marks the state dirty", () => {
    const view = describeGit(status({ staged: 2, modified: 1, untracked: 4 }));
    expect(view.dirty).toBe(true);
    expect(view.title).toBe("main · in sync with its upstream · 2 staged, 1 modified, 4 untracked");
  });

  it("marks a conflict above everything else", () => {
    const view = describeGit(status({ conflicted: 1, modified: 3 }));
    expect(view.conflicted).toBe(true);
    expect(view.title).toContain("1 in conflict, 3 modified");
  });

  it("shows a detached HEAD by its commit", () => {
    const view = describeGit(status({ branch: null, detached: true, hasUpstream: false }));
    expect(view.label).toBe("detached 6004959");
    expect(describeGit(status({ branch: null, detached: true, commit: null })).label).toBe("detached");
  });

  it("marks counts that were cut as at least that many", () => {
    const view = describeGit(status({ modified: 12, truncated: true }));
    expect(view.title).toContain("12+ modified");
  });
});

describe("the git store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockReset();
    git.watch(null);
  });
  afterEach(() => vi.useRealTimers());

  it("asks for the state of the folder a moment after it changes", async () => {
    vi.mocked(invoke).mockResolvedValue(status());

    git.watch("/srv/app");
    expect(invoke).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(300);

    expect(invoke).toHaveBeenCalledWith("git_status", { cwd: "/srv/app" });
    expect(git.status?.branch).toBe("main");
  });

  it("asks once for a burst of changes, about the last folder", async () => {
    vi.mocked(invoke).mockResolvedValue(status());

    git.watch("/a");
    git.watch("/b");
    git.watch("/c");
    await vi.advanceTimersByTimeAsync(300);

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("git_status", { cwd: "/c" });
  });

  it("drops the answer for a folder that has already been left", async () => {
    let answerFirst: (value: GitStatus) => void = () => {};
    vi.mocked(invoke)
      .mockImplementationOnce(() => new Promise<GitStatus>((resolve) => (answerFirst = resolve)))
      .mockResolvedValueOnce(status({ branch: "second" }));

    git.watch("/first");
    await vi.advanceTimersByTimeAsync(300);
    git.watch("/second");
    await vi.advanceTimersByTimeAsync(300);
    answerFirst(status({ branch: "first" }));
    await vi.advanceTimersByTimeAsync(0);

    expect(git.status?.branch).toBe("second");
  });

  it("shows nothing when there is no folder to follow, and clears what was shown", async () => {
    vi.mocked(invoke).mockResolvedValue(status());
    git.watch("/srv/app");
    await vi.advanceTimersByTimeAsync(300);
    expect(git.status).not.toBeNull();

    git.watch(null);

    expect(git.status).toBeNull();
    await vi.advanceTimersByTimeAsync(300);
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("shows nothing when git cannot be asked", async () => {
    vi.mocked(invoke).mockRejectedValue("boom");

    git.watch("/srv/app");
    await vi.advanceTimersByTimeAsync(300);

    expect(git.status).toBeNull();
  });

  it("asks again for the same folder on request", async () => {
    vi.mocked(invoke).mockResolvedValue(status());
    git.watch("/srv/app");
    await vi.advanceTimersByTimeAsync(300);

    git.refresh();
    await vi.advanceTimersByTimeAsync(300);

    expect(invoke).toHaveBeenCalledTimes(2);
  });
});
