import type { VerseWithTranslations } from "@/lib/bindings";

/**
 * A first guess of a verse's height before it is measured, from the length
 * of its texts. Closer guesses mean less jumping of the scrollbar.
 */
export function estimateVerseHeight(verse: VerseWithTranslations | undefined): number {
  if (!verse) return 160;
  const arabicLines = Math.ceil(verse.text_uthmani.length / 48);
  const translationLines = verse.translations.reduce(
    (lines, translation) => lines + Math.ceil(translation.text.length / 85),
    0,
  );
  return 60 + arabicLines * 62 + translationLines * 27 + verse.translations.length * 12;
}
