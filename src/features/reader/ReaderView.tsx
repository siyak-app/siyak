import { useParams } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { type RefObject, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import type { TextDirection } from "@/lib/bindings";
import { useSurah, useTranslations, useVerses } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { SurahHeader } from "./SurahHeader";
import { surahName } from "./surahName";
import { VerseItem } from "./VerseItem";
import { estimateVerseHeight } from "./verseLayout";

/** Verses rendered beyond the visible ones, above and below. */
const OVERSCAN = 6;

/**
 * Center pane: the surah in the URL. Remounted per surah, so each surah gets
 * a fresh virtualizer.
 */
export function ReaderView() {
  const { surah } = useParams({ from: "/read/$surah" });
  return <SurahReader key={surah} surah={surah} />;
}

/**
 * Offset of `list` inside its scroll container (the virtualizer's
 * `scrollMargin`), kept current while `header` above it changes size.
 */
function useListOffset(
  list: RefObject<HTMLElement | null>,
  header: RefObject<HTMLElement | null>,
): number {
  const [offset, setOffset] = useState(0);
  useLayoutEffect(() => {
    const update = () => {
      setOffset(list.current?.offsetTop ?? 0);
    };
    update();
    if (!header.current || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(update);
    observer.observe(header.current);
    return () => {
      observer.disconnect();
    };
  }, [list, header]);
  return offset;
}

function SurahReader({ surah }: { surah: number }) {
  const { t, i18n } = useTranslation();
  const info = useSurah(surah);
  const selected = useUiStore((state) => state.preferences["reader.translations"]);
  const installed = useTranslations();
  const verses = useVerses(surah, selected);
  const items = useMemo(() => verses.data ?? [], [verses.data]);

  const shown = useMemo(() => {
    const byId = new Map((installed.data ?? []).map((pack) => [pack.id, pack]));
    return selected.flatMap((id) => byId.get(id) ?? []);
  }, [installed.data, selected]);
  const directions = useMemo(
    () =>
      Object.fromEntries(shown.map((pack) => [pack.id, pack.dir])) as Record<string, TextDirection>,
    [shown],
  );

  const scrollRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const listOffset = useListOffset(listRef, headerRef);

  // The virtualizer re-renders on scroll by design; the compiler skipping
  // this component is expected.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => estimateVerseHeight(items[index]),
    getItemKey: (index) => items[index]?.id ?? index,
    overscan: OVERSCAN,
    scrollMargin: listOffset,
  });

  const name = info.data ? surahName(info.data, i18n.resolvedLanguage) : String(surah);

  return (
    <div ref={scrollRef} className="relative h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl px-8 pt-8 pb-16">
        <div ref={headerRef}>
          {info.data && <SurahHeader surah={info.data} translations={shown} />}
          {info.isError && <ErrorMessage error={info.error} />}
          {verses.isPending && <p className="text-muted-foreground">{t("common.loading")}</p>}
          {verses.isError && <ErrorMessage error={verses.error} />}
          {verses.data?.length === 0 && (
            <p className="py-6 text-muted-foreground">{t("reader.no_verses")}</p>
          )}
        </div>
        <div
          ref={listRef}
          role="list"
          aria-label={t("reader.verse_list_label", { name })}
          className="relative"
          style={{ height: virtualizer.getTotalSize() }}
        >
          {virtualizer.getVirtualItems().map((row) => {
            const verse = items[row.index];
            if (!verse) return null;
            return (
              <div
                key={row.key}
                ref={virtualizer.measureElement}
                data-index={row.index}
                role="listitem"
                aria-setsize={items.length}
                aria-posinset={row.index + 1}
                className="absolute inset-x-0 top-0"
                style={{ transform: `translateY(${String(row.start - listOffset)}px)` }}
              >
                <VerseItem verse={verse} directions={directions} />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
