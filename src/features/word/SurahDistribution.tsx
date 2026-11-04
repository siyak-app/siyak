import { useState } from "react";
import { useTranslation } from "react-i18next";

import { surahName } from "@/features/reader/surahName";
import type { SurahCount } from "@/lib/bindings";
import { useSurahs } from "@/lib/queries";

/** Surahs in the Quran; a surah without occurrences keeps its place on the axis. */
const SURAH_COUNT = 114;

/**
 * Occurrences per surah (spec §13.3): one bar per surah in Mushaf order, so
 * the shape shows where a root gathers. Hovering names a bar; the table below
 * holds the same numbers for anyone who cannot use the bars.
 */
export function SurahDistribution({ counts }: { counts: readonly SurahCount[] }) {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  const [hovered, setHovered] = useState<number | null>(null);
  const language = i18n.resolvedLanguage;
  const nameOf = (number: number) => {
    const surah = surahs.data?.find((item) => item.number === number);
    return surah ? surahName(surah, language) : String(number);
  };
  const byNumber = new Map(counts.map((item) => [item.surah, item.occurrences]));
  const highest = Math.max(1, ...counts.map((item) => item.occurrences));
  const numbers = Array.from({ length: SURAH_COUNT }, (_, index) => index + 1);
  return (
    <section className="grid gap-1.5">
      <div className="flex items-baseline justify-between gap-2 text-xs">
        <h3 className="font-medium">{t("concordance.by_surah")}</h3>
        <p role="status" className="text-muted-foreground tabular-nums">
          {hovered === null
            ? t("concordance.surah_count", { count: counts.length })
            : t("concordance.surah_bar", {
                number: hovered,
                name: nameOf(hovered),
                count: byNumber.get(hovered) ?? 0,
              })}
        </p>
      </div>
      <div
        aria-hidden
        className="flex h-16 items-end border-b border-border"
        onMouseLeave={() => {
          setHovered(null);
        }}
      >
        {numbers.map((number) => {
          const occurrences = byNumber.get(number) ?? 0;
          return (
            <div
              key={number}
              className="flex h-full min-w-0 flex-1 items-end px-px hover:bg-muted"
              onMouseEnter={() => {
                setHovered(number);
              }}
            >
              {occurrences > 0 && (
                <div
                  className="w-full rounded-t-[2px] bg-primary"
                  // A single occurrence must stay visible next to hundreds.
                  style={{ height: `max(2px, ${String((occurrences / highest) * 100)}%)` }}
                />
              )}
            </div>
          );
        })}
      </div>
      <details className="text-xs">
        <summary className="cursor-pointer text-muted-foreground">
          {t("concordance.show_table")}
        </summary>
        <table className="mt-1 w-full text-start">
          <thead className="text-muted-foreground">
            <tr>
              <th scope="col" className="text-start font-normal">
                {t("concordance.surah")}
              </th>
              <th scope="col" className="text-end font-normal">
                {t("concordance.count")}
              </th>
            </tr>
          </thead>
          <tbody>
            {counts.map((item) => (
              <tr key={item.surah}>
                <td>{`${String(item.surah)}. ${nameOf(item.surah)}`}</td>
                <td className="text-end tabular-nums">{item.occurrences}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </details>
    </section>
  );
}
