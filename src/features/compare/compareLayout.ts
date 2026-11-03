import type { TafsirInfo, TafsirPassage, TranslationInfo } from "@/lib/bindings";

/** A column of the compare table. */
export type CompareColumn =
  { kind: "translation"; pack: TranslationInfo } | { kind: "tafsir"; pack: TafsirInfo };

/**
 * The columns to show, in order. Until the user has chosen (`null`), the
 * reader's translations and the first tafsir; packs no longer installed are
 * skipped.
 */
export function compareColumns(
  stored: readonly string[] | null,
  readerTranslations: readonly string[],
  translations: readonly TranslationInfo[],
  tafsirs: readonly TafsirInfo[],
): CompareColumn[] {
  const firstTafsir = tafsirs[0]?.id;
  const ids = stored ?? [...readerTranslations, ...(firstTafsir ? [firstTafsir] : [])];
  return ids.flatMap((id): CompareColumn[] => {
    const translation = translations.find((pack) => pack.id === id);
    if (translation) return [{ kind: "translation", pack: translation }];
    const tafsir = tafsirs.find((pack) => pack.id === id);
    return tafsir ? [{ kind: "tafsir", pack: tafsir }] : [];
  });
}

/** One tafsir cell of the table: a passage (or a gap) over `span` rows. */
export interface TafsirCell {
  /** `null` where the tafsir has no commentary. */
  passage: TafsirPassage | null;
  span: number;
}

const position = (verse: { surah: number; ayah: number }) => verse.surah * 1000 + verse.ayah;

/**
 * Lays one tafsir's passages over the rows of the table, so that a passage
 * covering several verses is one cell spanning their rows. Returns one entry
 * per row: the cell starting there, or `null` where a cell from above continues.
 * Consecutive verses without commentary share one gap cell.
 */
export function tafsirCells(
  passages: readonly TafsirPassage[],
  verses: readonly { surah: number; ayah: number }[],
): (TafsirCell | null)[] {
  const cells: (TafsirCell | null)[] = [];
  let row = 0;
  while (row < verses.length) {
    const here = position(verses[row] ?? { surah: 0, ayah: 0 });
    const passage = passages.find(
      (item) => position(item.verse_from) <= here && here <= position(item.verse_to),
    );
    let span = 1;
    while (row + span < verses.length) {
      const next = position(verses[row + span] ?? { surah: 0, ayah: 0 });
      const nextPassage = passages.find(
        (item) => position(item.verse_from) <= next && next <= position(item.verse_to),
      );
      if (nextPassage !== passage) break;
      span += 1;
    }
    cells.push({ passage: passage ?? null, span });
    for (let covered = 1; covered < span; covered += 1) cells.push(null);
    row += span;
  }
  return cells;
}
