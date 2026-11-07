import { ChevronsDownUpIcon, ChevronsUpDownIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { Surah, TranslationInfo } from "@/lib/bindings";

import { surahName } from "./surahName";
import { TranslationPicker } from "./TranslationPicker";
import { translationTitle } from "./translationTitle";

/**
 * Top of the reader: the surah's names and facts, the titles of the shown
 * translations (once per surah rather than on every verse) and the basmala,
 * which is a header for every surah except 1 (where it is verse 1) and 9
 * (spec §5.1, §13.3). In reading mode it also opens or closes every verse
 * card's translations at once (`expandAll`).
 */
export function SurahHeader({
  surah,
  translations,
  expandAll,
}: {
  surah: Surah;
  /** The translations shown, in display order. */
  translations: readonly TranslationInfo[];
  /** Whether every card is open, and the toggle of all of them. */
  expandAll?: { expanded: boolean; onToggle: () => void };
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
      <div className="mt-3 flex items-center justify-between gap-4">
        <p className="text-sm text-muted-foreground">
          {titles.length === 0
            ? t("reader.arabic_only")
            : t("reader.translation_titles", { count: titles.length, titles: titles.join(" · ") })}
        </p>
        <div className="flex shrink-0 items-center gap-1">
          {expandAll && (
            <Button variant="ghost" size="sm" onClick={expandAll.onToggle}>
              {expandAll.expanded ? (
                <ChevronsDownUpIcon aria-hidden />
              ) : (
                <ChevronsUpDownIcon aria-hidden />
              )}
              {expandAll.expanded ? t("reader.collapse_all") : t("reader.expand_all")}
            </Button>
          )}
          <TranslationPicker />
        </div>
      </div>
      {surah.bismillah !== null && (
        <p dir="rtl" lang="ar" className="arabic-text mt-6 text-center" data-testid="basmala">
          {surah.bismillah}
        </p>
      )}
    </header>
  );
}
