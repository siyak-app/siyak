import { useParams } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSurahs, useVerses } from "@/lib/queries";

import { DEFAULT_TRANSLATION_IDS } from "./defaults";
import { surahName } from "./surahName";
import { VerseItem } from "./VerseItem";

/**
 * Center pane: the verses of the surah in the URL.
 * TODO(M2): virtualized list, translation selection and verse actions.
 */
export function ReaderView() {
  const { surah } = useParams({ from: "/read/$surah" });
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  const verses = useVerses(surah, DEFAULT_TRANSLATION_IDS);
  const info = surahs.data?.find((s) => s.number === surah);

  return (
    <ScrollArea className="h-full">
      <div className="mx-auto max-w-3xl px-8 py-8">
        {info && (
          <header className="mb-4 flex items-baseline justify-between gap-4 border-b pb-4">
            <div>
              <h1 className="text-xl font-semibold">
                {info.number}. {surahName(info, i18n.resolvedLanguage)}
              </h1>
              <p className="flex gap-1.5 text-sm text-muted-foreground">
                <span>
                  {t(info.revelation_type === "meccan" ? "reader.meccan" : "reader.medinan")}
                </span>
                <span className="before:me-1.5 before:content-['·']">
                  {t("reader.ayah_count", { count: info.ayah_count })}
                </span>
              </p>
            </div>
            <p dir="rtl" lang="ar" className="font-quran text-3xl">
              {info.name_ar}
            </p>
          </header>
        )}
        {verses.isPending && <p className="text-muted-foreground">{t("common.loading")}</p>}
        {verses.isError && <ErrorMessage error={verses.error} />}
        {verses.data?.length === 0 && (
          <p className="text-muted-foreground">{t("reader.no_verses")}</p>
        )}
        {verses.data?.map((verse) => (
          <VerseItem key={verse.id} verse={verse} />
        ))}
      </div>
    </ScrollArea>
  );
}
