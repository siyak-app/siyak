import type { VerseWithTranslations } from "@/lib/bindings";

/**
 * A first guess of a verse's height before it is measured, from the length
 * of its texts. Closer guesses mean less jumping of the scrollbar. A reading
 * mode `card` adds its medallion and toggle; a `collapsed` one hides its
 * translations.
 */
export function estimateVerseHeight(
  verse: VerseWithTranslations | undefined,
  { card = false, collapsed = false }: { card?: boolean; collapsed?: boolean } = {},
): number {
  if (!verse) return 160;
  const chrome = card ? 90 : 0;
  const arabicLines = Math.ceil(verse.text_uthmani.length / 48);
  if (collapsed) return 60 + chrome + arabicLines * 62;
  const translationLines = verse.translations.reduce(
    (lines, translation) => lines + Math.ceil(translation.text.length / 85),
    0,
  );
  return 60 + chrome + arabicLines * 62 + translationLines * 27 + verse.translations.length * 12;
}
