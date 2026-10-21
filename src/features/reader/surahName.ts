import type { Surah } from "@/lib/bindings";

/** The surah's name in the interface language (Turkish name or English transliteration). */
export function surahName(surah: Surah, language: string | undefined): string {
  return language === "tr" ? surah.name_tr : surah.name_en;
}
