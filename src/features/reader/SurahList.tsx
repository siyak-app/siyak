import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSurahs } from "@/lib/queries";

import { surahName } from "./surahName";

const ACTIVE_LINK_PROPS = { className: "bg-primary/10 text-primary hover:bg-primary/15" };

/** Left-pane navigation: every surah, linking to the reader. */
export function SurahList() {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();

  return (
    <nav aria-labelledby="surah-list-heading" className="flex h-full flex-col">
      <h2
        id="surah-list-heading"
        className="px-4 pt-4 pb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase"
      >
        {t("navigation.surahs")}
      </h2>
      {surahs.isError && <ErrorMessage error={surahs.error} />}
      <ScrollArea className="min-h-0 flex-1">
        <ol className="px-2 pb-4">
          {surahs.data?.map((surah) => (
            <li key={surah.number}>
              <Link
                to="/read/$surah"
                params={{ surah: surah.number }}
                className="flex items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
                activeProps={ACTIVE_LINK_PROPS}
              >
                <span className="w-7 text-end text-xs text-muted-foreground tabular-nums">
                  {surah.number}
                </span>
                <span className="flex-1 truncate">{surahName(surah, i18n.resolvedLanguage)}</span>
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
