import { Link } from "@tanstack/react-router";
import { useDeferredValue, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSurahs } from "@/lib/queries";

import { matchesSurah } from "./surahFilter";
import { surahName } from "./surahName";

const ACTIVE_LINK_PROPS = { className: "bg-primary/10 hover:bg-primary/15" };

/**
 * Left-pane navigation: every surah with its number, names, verse count and
 * place of revelation, linking to the reader. The filter matches the number
 * and the Turkish, English or Arabic name.
 */
export function SurahList() {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const filterId = useId();
  const visible = surahs.data?.filter((surah) => matchesSurah(surah, deferredQuery));

  return (
    <nav aria-labelledby="surah-list-heading" className="flex h-full flex-col">
      <h2
        id="surah-list-heading"
        className="px-4 pt-4 pb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase"
      >
        {t("navigation.surahs")}
      </h2>
      <div className="px-3 pb-2">
        <label htmlFor={filterId} className="sr-only">
          {t("navigation.filter_label")}
        </label>
        <input
          id={filterId}
          type="search"
          value={query}
          placeholder={t("navigation.filter_placeholder")}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
          className="h-8 w-full rounded-md border bg-background px-2.5 text-sm placeholder:text-muted-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        />
      </div>
      {surahs.isError && <ErrorMessage error={surahs.error} />}
      {visible?.length === 0 && (
        <p className="px-4 py-2 text-sm text-muted-foreground">{t("navigation.no_matches")}</p>
      )}
      <ScrollArea className="min-h-0 flex-1">
        <ol className="px-2 pb-4">
          {visible?.map((surah) => (
            <li key={surah.number}>
              <Link
                to="/read/$surah"
                params={{ surah: surah.number }}
                className="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-muted"
                activeProps={ACTIVE_LINK_PROPS}
              >
                <span className="w-7 text-end text-xs text-muted-foreground tabular-nums">
                  {surah.number}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm">
                    {surahName(surah, i18n.resolvedLanguage)}
                  </span>
                  <span className="block text-xs text-muted-foreground">
                    {t(surah.revelation_type === "meccan" ? "reader.meccan" : "reader.medinan")}
                    {" · "}
                    {t("reader.ayah_count", { count: surah.ayah_count })}
                  </span>
                </span>
                <span dir="rtl" lang="ar" className="font-quran text-base">
                  {surah.name_ar}
                </span>
              </Link>
            </li>
          ))}
        </ol>
      </ScrollArea>
    </nav>
  );
}
