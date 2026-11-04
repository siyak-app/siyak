import { useTranslation } from "react-i18next";

import { surahName } from "@/features/reader/surahName";
import { verseRange } from "@/features/tafsir/tafsirRefs";
import type { CollectionItem, Surah, VerseRef } from "@/lib/bindings";
import { useSurahs } from "@/lib/queries";

/** A verse range with the surah's name, e.g. "Bakara 2:153–157". */
export function namedRange(
  surahs: readonly Surah[] | undefined,
  from: VerseRef,
  to: VerseRef,
  language: string | undefined,
): string {
  const surah = surahs?.find((item) => item.number === from.surah);
  const range = verseRange(from, to);
  return surah ? `${surahName(surah, language)} ${range}` : range;
}

/** The heading of an item, for screen readers and for reordering. */
export function useItemName(item: CollectionItem): string {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  const { content } = item;
  const range = (from: VerseRef, to: VerseRef) =>
    namedRange(surahs.data, from, to, i18n.resolvedLanguage);
  switch (content.kind) {
    case "verse_range": {
      const first = content.verses[0];
      const last = content.verses.at(-1);
      return first && last ? range(first, last) : "";
    }
    case "note":
      return content.note ? range(content.note.from, content.note.to) : t("collections.note_item");
    case "root":
      return item.item.kind === "root" ? item.item.root : "";
    case "lemma":
      return item.item.kind === "lemma" ? item.item.lemma.replace(/\d+$/u, "") : "";
    case "tafsir":
      return content.verse
        ? `${t("collections.tafsir_item")} · ${range(content.verse, content.verse)}`
        : t("collections.tafsir_item");
  }
}
