/** What a search found: the position of the active match and how many there are. */
export interface SearchResults {
  index: number;
  count: number;
}

export const NO_RESULTS: SearchResults = { index: -1, count: 0 };

/** The text next to the search box. */
export function describeResults(results: SearchResults, hasTerm: boolean): string {
  if (!hasTerm) return "";
  if (results.count === 0) return "No matches";
  // The addon stops marking at its highlight limit and then reports no active position.
  if (results.index < 0) return `${results.count}+ matches`;
  return `${results.index + 1} of ${results.count}`;
}

/** `null` when `term` is a usable regular expression, else why it is not. */
export function regexProblem(term: string): string | null {
  if (!term) return null;
  try {
    new RegExp(term);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message.replace(/^Invalid regular expression: /, "") : "Invalid expression";
  }
}

export interface MatchColors {
  matchBackground: string;
  matchBorder: string;
  matchOverviewRuler: string;
  activeMatchBackground: string;
  activeMatchBorder: string;
  activeMatchColorOverviewRuler: string;
}

/** The addon wants `#RRGGBB`, so these are fixed values per theme rather than CSS variables. */
export function matchColors(dark: boolean): MatchColors {
  return dark
    ? {
        matchBackground: "#5c4b00",
        matchBorder: "#8a7100",
        matchOverviewRuler: "#facc15",
        activeMatchBackground: "#b45309",
        activeMatchBorder: "#fbbf24",
        activeMatchColorOverviewRuler: "#f59e0b",
      }
    : {
        matchBackground: "#fde68a",
        matchBorder: "#fbbf24",
        matchOverviewRuler: "#f59e0b",
        activeMatchBackground: "#fb923c",
        activeMatchBorder: "#c2410c",
        activeMatchColorOverviewRuler: "#ea580c",
      };
}
