import { useParams } from "@tanstack/react-router";
import { useVirtualizer, type Virtualizer } from "@tanstack/react-virtual";
import { type RefObject, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import type { VerseWithTranslations } from "@/lib/bindings";
import { useSurah, useTranslations, useVerses } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { recallTopAyah, rememberTopAyah } from "./scrollMemory";
import { SurahHeader } from "./SurahHeader";
import { surahName } from "./surahName";
import { translationTitle } from "./translationTitle";
import { type ShownPack, VerseItem } from "./VerseItem";
import { estimateVerseHeight } from "./verseLayout";

/** Verses rendered beyond the visible ones, above and below. */
const OVERSCAN = 6;

/** How long a verse opened by a deep link stays highlighted. */
const HIGHLIGHT_MS = 2500;

/** Wait for the reader to settle before storing the last read position. */
const LAST_POSITION_DELAY_MS = 1000;

/**
 * Center pane: `/read/:surah` or `/read/:surah/:ayah`. Remounted per surah,
 * so each surah gets a fresh virtualizer.
 */
export function ReaderView() {
  const { surah = 1, ayah } = useParams({ strict: false });
  return <SurahReader key={surah} surah={surah} ayah={ayah} />;
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

/** The verse at the top of the visible area. */
function topVerse(
  virtualizer: Virtualizer<HTMLDivElement, Element>,
  items: readonly VerseWithTranslations[],
): VerseWithTranslations | undefined {
  const offset = virtualizer.scrollOffset ?? 0;
  const row = virtualizer.getVirtualItems().find((item) => item.end > offset + 1);
  return row ? items[row.index] : undefined;
}

/**
 * Keeps track of the verse at the top while the user scrolls: remembered
 * per surah for this session (switching surahs back and forth) and stored as
 * `reader.last_position`, which the app opens on the next launch.
 */
function useReadingPosition(
  surah: number,
  scrollRef: RefObject<HTMLDivElement | null>,
  virtualizer: Virtualizer<HTMLDivElement, Element>,
  items: readonly VerseWithTranslations[],
) {
  useEffect(() => {
    const element = scrollRef.current;
    if (!element || items.length === 0) return;
    let frame = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const verse = topVerse(virtualizer, items);
        if (!verse) return;
        rememberTopAyah(surah, verse.ayah);
        clearTimeout(timer);
        timer = setTimeout(() => {
          const { preferences, setPreference } = useUiStore.getState();
          const last = preferences["reader.last_position"];
          if (last?.surah === surah && last.ayah === verse.ayah) return;
          setPreference("reader.last_position", { surah, ayah: verse.ayah });
        }, LAST_POSITION_DELAY_MS);
      });
    };
    update();
    element.addEventListener("scroll", update, { passive: true });
    return () => {
      element.removeEventListener("scroll", update);
      cancelAnimationFrame(frame);
      clearTimeout(timer);
    };
  }, [surah, scrollRef, virtualizer, items]);
}

function SurahReader({ surah, ayah }: { surah: number; ayah: number | undefined }) {
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
  const language = i18n.resolvedLanguage;
  const packs = useMemo(
    () =>
      Object.fromEntries(
        shown.map((pack) => [pack.id, { dir: pack.dir, title: translationTitle(pack, language) }]),
      ) as Record<string, ShownPack>,
    [shown, language],
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

  // A deep link scrolls to its verse and highlights it briefly; otherwise the
  // surah opens where it was left in this session.
  const [highlighted, setHighlighted] = useState<number | null>(null);
  useEffect(() => {
    const target = ayah ?? recallTopAyah(surah);
    const index = items.findIndex((verse) => verse.ayah === target);
    if (index < 0) return;
    virtualizer.scrollToIndex(index, { align: "start" });
    if (ayah === undefined) return;
    setHighlighted(ayah);
    const timer = setTimeout(() => {
      setHighlighted(null);
    }, HIGHLIGHT_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [surah, ayah, items, virtualizer]);

  useReadingPosition(surah, scrollRef, virtualizer, items);

  const name = info.data ? surahName(info.data, language) : String(surah);
  const noSuchVerse = ayah !== undefined && info.data !== undefined && ayah > info.data.ayah_count;

  return (
    <div ref={scrollRef} className="relative h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl px-8 pt-8 pb-16">
        <div ref={headerRef}>
          {info.data && <SurahHeader surah={info.data} translations={shown} />}
          {info.isError && <ErrorMessage error={info.error} />}
          {noSuchVerse && (
            <p role="status" className="py-3 text-muted-foreground">
              {t("reader.verse_not_found", { surah, ayah })}
            </p>
          )}
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
                <VerseItem
                  verse={verse}
                  packs={packs}
                  surahName={name}
                  highlighted={verse.ayah === highlighted}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
