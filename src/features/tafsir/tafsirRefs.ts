import type { Surah, TafsirParagraph, VerseRef } from "@/lib/bindings";
import type { VersePosition } from "@/lib/settings";

/**
 * The global id of a verse, from the verse counts of the surahs before it
 * (ids are 1..6236 in Mushaf order and never change, spec §6.2).
 */
export function verseIdOf(surahs: readonly Surah[], verse: VersePosition): number {
  return surahs
    .filter((surah) => surah.number < verse.surah)
    .reduce((id, surah) => id + surah.ayah_count, verse.ayah);
}

/** The position of a global verse id; `null` if there is no such verse. */
export function verseOfId(surahs: readonly Surah[], id: number): VersePosition | null {
  let rest = id;
  for (const surah of surahs) {
    if (rest <= surah.ayah_count) return rest >= 1 ? { surah: surah.number, ayah: rest } : null;
    rest -= surah.ayah_count;
  }
  return null;
}

/** A verse range such as "2:153–157"; ranges normally stay within a surah. */
export function verseRange(from: VerseRef, to: VerseRef): string {
  const start = `${String(from.surah)}:${String(from.ayah)}`;
  if (from.surah === to.surah && from.ayah === to.ayah) return start;
  if (from.surah !== to.surah) return `${start}–${String(to.surah)}:${String(to.ayah)}`;
  return `${start}–${String(to.ayah)}`;
}

/** Where a paragraph is in the printed edition, e.g. "1/113", or `null` if unknown. */
export function editionPage(part: string | null, page: number | null): string | null {
  if (page === null) return part;
  return part === null ? String(page) : `${part}/${String(page)}`;
}

/** The page to mark on each paragraph: only where a new page starts, else `null`. */
export function pageMarks(paragraphs: readonly TafsirParagraph[]): (string | null)[] {
  let previous: string | null = null;
  return paragraphs.map(({ part, page }) => {
    const current = editionPage(part, page);
    if (current === null || current === previous) return null;
    previous = current;
    return current;
  });
}
