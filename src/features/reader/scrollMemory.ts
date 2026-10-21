/**
 * The verse at the top of each surah when the reader last left it, for this
 * session only: switching to another surah and back returns to the same
 * place. The persistent position is `reader.last_position`.
 */
const topAyahBySurah = new Map<number, number>();

/** Records the verse at the top of `surah`. */
export function rememberTopAyah(surah: number, ayah: number): void {
  topAyahBySurah.set(surah, ayah);
}

/** The verse that was at the top of `surah`, if it was read in this session. */
export function recallTopAyah(surah: number): number | undefined {
  return topAyahBySurah.get(surah);
}
