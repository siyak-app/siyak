import type { VerseWithTranslations } from "@/lib/bindings";

/**
 * The shown translations of a verse as plain text. With more than one, each
 * is preceded by its title so the copied text says whose translation it is.
 */
export function translationsText(
  verse: VerseWithTranslations,
  titles: Readonly<Record<string, string>>,
): string {
  if (verse.translations.length === 1) return verse.translations[0]?.text ?? "";
  return verse.translations
    .map(({ pack_id, text }) => `${titles[pack_id] ?? pack_id}\n${text}`)
    .join("\n\n");
}

/** A verse number such as "2:153". */
export function verseNumber(verse: { surah: number; ayah: number }): string {
  return `${String(verse.surah)}:${String(verse.ayah)}`;
}

/** A verse reference such as "Al-Baqara 2:153" or "Bakara 2:153". */
export function verseReference(surahName: string, verse: { surah: number; ayah: number }): string {
  return `${surahName} ${verseNumber(verse)}`;
}
