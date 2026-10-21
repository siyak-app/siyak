import type { Surah } from "@/lib/bindings";
import { fold, foldName } from "@/lib/fold";

/**
 * Whether `surah` matches the filter text: its number (prefix), or its
 * Turkish, English or Arabic name, ignoring case, accents, harakat and
 * transliteration differences.
 */
export function matchesSurah(surah: Surah, query: string): boolean {
  const text = query.trim();
  if (text === "") return true;
  if (/^\d+$/u.test(text)) return String(surah.number).startsWith(text);
  const folded = fold(text);
  const name = foldName(text);
  return (
    fold(surah.name_tr).includes(folded) ||
    fold(surah.name_ar).includes(folded) ||
    foldName(surah.name_en).includes(name) ||
    foldName(surah.name_tr).includes(name)
  );
}
