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
