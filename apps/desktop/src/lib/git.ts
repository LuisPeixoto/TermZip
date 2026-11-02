/** How the status bar shows the git state of a folder. */
import type { GitStatus } from "./api";

export interface GitView {
  /** The branch, or `detached` and where HEAD is. */
  label: string;
  /** How far the branch is from its upstream: `↑2 ↓1`, or empty. */
  sync: string;
  /** Anything differs from HEAD. */
  dirty: boolean;
  conflicted: boolean;
  /** Everything, for the tooltip. */
  title: string;
}

const plural = (count: number, one: string, many = `${one}s`) => `${count} ${count === 1 ? one : many}`;

export function describeGit(status: GitStatus): GitView {
  const label = status.detached ? `detached ${status.commit ?? ""}`.trim() : (status.branch ?? "(no branch)");
  const sync = [status.ahead > 0 ? `↑${status.ahead}` : "", status.behind > 0 ? `↓${status.behind}` : ""]
    .filter(Boolean)
    .join(" ");
  // A cut output only has a lower bound, which a trailing + says.
  const more = status.truncated ? "+" : "";
  const changes = [
    status.conflicted > 0 ? `${status.conflicted}${more} in conflict` : "",
    status.staged > 0 ? `${status.staged}${more} staged` : "",
    status.modified > 0 ? `${status.modified}${more} modified` : "",
    status.untracked > 0 ? `${status.untracked}${more} untracked` : "",
  ].filter(Boolean);
  const dirty = changes.length > 0;
  const upstream = !status.hasUpstream
    ? "no upstream"
    : status.ahead + status.behind === 0
      ? "in sync with its upstream"
      : [
          status.ahead > 0 ? `${plural(status.ahead, "commit")} ahead` : "",
          status.behind > 0 ? `${plural(status.behind, "commit")} behind` : "",
        ]
          .filter(Boolean)
          .join(", ");
  return {
    label,
    sync,
    dirty,
    conflicted: status.conflicted > 0,
    title: [label, upstream, dirty ? changes.join(", ") : "no changes"].join(" · "),
  };
}
