/** Modes of the search screen (spec §7.1). */
export const SEARCH_MODES = ["all", "text", "root", "meaning"] as const;

/** A mode of the search screen. */
export type SearchMode = (typeof SEARCH_MODES)[number];

/** The search screen's URL parameters. */
export interface SearchParams {
  q: string;
  mode: SearchMode;
}

/** Reads `/search` parameters; anything unusable falls back to an empty All search. */
export function parseSearchParams(raw: Record<string, unknown>): SearchParams {
  // The router parses URL values as JSON, so a hand-written `q=36` is a number.
  const q = typeof raw.q === "string" ? raw.q : typeof raw.q === "number" ? String(raw.q) : "";
  const mode = SEARCH_MODES.find((value) => value === raw.mode) ?? "all";
  return { q, mode };
}
