import { useTranslation } from "react-i18next";

import type { Surah, TranslationInfo } from "@/lib/bindings";

import { surahName } from "./surahName";
import { translationTitle } from "./translationTitle";

/**
 * Top of the reader: the surah's names and facts, the titles of the shown
 * translations (once per surah rather than on every verse) and the basmala,
 * which is a header for every surah except 1 (where it is verse 1) and 9
 * (spec §5.1, §13.3).
 */
export function SurahHeader({
  surah,
  translations,
}: {
  surah: Surah;
  /** The translations shown, in display order. */
  translations: readonly TranslationInfo[];
}) {
  const { t, i18n } = useTranslation();
  const titles = translations.map((info) => translationTitle(info, i18n.resolvedLanguage));
  return (
    <header className="mb-2 border-b pb-4">
      <div className="flex items-baseline justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">
            {surah.number}. {surahName(surah, i18n.resolvedLanguage)}
          </h1>
          <p className="text-sm text-muted-foreground">
            {t(surah.revelation_type === "meccan" ? "reader.meccan" : "reader.medinan")}
            {" · "}
            {t("reader.ayah_count", { count: surah.ayah_count })}
          </p>
        </div>
        <p dir="rtl" lang="ar" className="font-quran text-3xl">
          {surah.name_ar}
        </p>
      </div>
      <p className="mt-3 text-sm text-muted-foreground">
        {titles.length === 0
          ? t("reader.arabic_only")
          : t("reader.translation_titles", { count: titles.length, titles: titles.join(" · ") })}
      </p>
      {surah.bismillah !== null && (
        <p dir="rtl" lang="ar" className="arabic-text mt-6 text-center" data-testid="basmala">
          {surah.bismillah}
        </p>
      )}
    </header>
  );
}
