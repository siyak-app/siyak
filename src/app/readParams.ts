/** Number of surahs in the Quran. */
export const SURAH_COUNT = 114;

/** The longest surah (al-Baqarah) has 286 verses. */
export const MAX_AYAH = 286;

/** A positive decimal integer without signs, spaces, leading zeros or fractions. */
const POSITIVE_INTEGER = /^[1-9]\d*$/u;

/** Parses the `:surah` segment of `/read/:surah[/:ayah]`: 1..114, or `null`. */
export function parseSurahParam(raw: string): number | null {
  if (!POSITIVE_INTEGER.test(raw)) return null;
  const surah = Number(raw);
  return surah <= SURAH_COUNT ? surah : null;
}

/**
 * Parses the `:ayah` segment: a verse number up to the longest surah's
 * length, or `null`. Whether the surah has that many verses is checked by
 * the reader, which knows the surah.
 */
export function parseAyahParam(raw: string): number | null {
  if (!POSITIVE_INTEGER.test(raw)) return null;
  const ayah = Number(raw);
  return ayah <= MAX_AYAH ? ayah : null;
}

/** A verse range of a surah, e.g. the `153-157` of `/compare/2/153-157`. */
export interface VerseRange {
  from: number;
  to: number;
}

/**
 * Parses the `:range` segment of `/compare/:surah/:range`: `255` or `153-157`,
 * or `null`. Whether the surah has those verses is checked by the screen.
 */
export function parseRangeParam(raw: string): VerseRange | null {
  const [first, last, ...rest] = raw.split("-");
  if (first === undefined || rest.length > 0) return null;
  const from = parseAyahParam(first);
  const to = last === undefined ? from : parseAyahParam(last);
  return from === null || to === null || to < from ? null : { from, to };
}

/** The `:range` segment for a range: `255` for one verse, else `153-157`. */
export function formatRangeParam({ from, to }: VerseRange): string {
  return from === to ? String(from) : `${String(from)}-${String(to)}`;
}
