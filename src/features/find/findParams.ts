/** The Find screen's URL parameters. */
export interface FindParams {
  /** The question last searched for; empty before the first search. */
  q: string;
}

/** Reads `/find` parameters; anything unusable falls back to no question. */
export function parseFindParams(raw: Record<string, unknown>): FindParams {
  // The router parses URL values as JSON, so a hand-written `q=36` is a number.
  const q = typeof raw.q === "string" ? raw.q : typeof raw.q === "number" ? String(raw.q) : "";
  return { q };
}
