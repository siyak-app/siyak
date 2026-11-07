import { cn } from "cn";
import { BookmarkIcon, ChevronDownIcon, NotebookPenIcon } from "lucide-react";
import { memo, type ReactNode, useId } from "react";
import { useTranslation } from "react-i18next";

import { isTrackOf, useAudioStore } from "@/features/audio/store";
import { VerseArabic } from "@/features/word/VerseArabic";
import type { TextDirection, VerseWithTranslations } from "@/lib/bindings";
import { useUiStore } from "@/stores/ui";

import { VerseActions } from "./VerseActions";

/** What a verse needs to know about a shown translation pack. */
export interface ShownPack {
  dir: TextDirection;
  /** Title in the interface language. */
  title: string;
}

/**
 * One verse: Arabic text followed by the selected translations, each in its
 * pack's language and direction, and the verse's actions. As a `list` row
 * (research mode) the translations are always shown; as a `card` (reading
 * mode) they open under the Arabic when the card is clicked. Memoized:
 * scrolling re-renders only the verses entering the virtualized window.
 */
export const VerseItem = memo(function VerseItem({
  verse,
  packs,
  surahName,
  variant = "list",
  expanded = false,
  onToggle,
  highlighted = false,
  tabStop = false,
  onCompare,
  bookmarked = false,
  noted = false,
  onBookmark,
  onPlay,
}: {
  verse: VerseWithTranslations;
  /** The shown translation packs by id. */
  packs: Readonly<Record<string, ShownPack>>;
  /** The surah's name in the interface language. */
  surahName: string;
  /** An always open row, or a card whose translations open on click. */
  variant?: "list" | "card";
  /** Whether a card shows its translations. */
  expanded?: boolean;
  /** Opens or closes a card's translations. */
  onToggle?: (verse: VerseWithTranslations) => void;
  /** Briefly marks the verse a deep link opened. */
  highlighted?: boolean;
  /** The verse list's one tab stop (roving focus). */
  tabStop?: boolean;
  /** Opens the verse on the compare screen. */
  onCompare?: (verse: VerseWithTranslations) => void;
  /** Whether the verse has a bookmark. */
  bookmarked?: boolean;
  /** Whether one of the user's notes touches the verse. */
  noted?: boolean;
  /** Toggles the verse's bookmark. */
  onBookmark?: (verse: VerseWithTranslations) => void;
  /** Plays or pauses the verse's recitation; only with an installed reciter. */
  onPlay?: (verse: VerseWithTranslations) => void;
}) {
  const { t } = useTranslation();
  const mode = useUiStore((state) => state.preferences["reader.actions"]);
  const selected = useUiStore(
    (state) =>
      state.selectedVerse?.surah === verse.surah && state.selectedVerse.ayah === verse.ayah,
  );
  // The verse loaded in the player (not while its surah's basmala plays).
  const current = useAudioStore((state) => isTrackOf(state.track, verse.surah, verse.ayah));
  const playing = useAudioStore(
    (state) => current && (state.status === "playing" || state.status === "loading"),
  );
  const titles = Object.fromEntries(Object.entries(packs).map(([id, pack]) => [id, pack.title]));
  const actions = (
    <VerseActions
      verse={verse}
      surahName={surahName}
      titles={titles}
      mode={mode}
      onCompare={onCompare}
      bookmarked={bookmarked}
      onBookmark={onBookmark}
      onPlay={onPlay}
      playing={playing}
    />
  );
  const marks = (
    <>
      {bookmarked && (
        <BookmarkIcon
          role="img"
          aria-label={t("verse_actions.bookmarked")}
          className="size-3.5 fill-current text-primary"
        />
      )}
      {noted && (
        <NotebookPenIcon
          role="img"
          aria-label={t("verse_actions.has_note")}
          className="size-3.5 text-muted-foreground"
        />
      )}
    </>
  );
  if (variant === "card") {
    return (
      <VerseCard
        verse={verse}
        packs={packs}
        expanded={expanded}
        onToggle={onToggle}
        marks={marks}
        actions={actions}
        labeled={mode === "labeled"}
        state={{ highlighted, selected, current, tabStop }}
      />
    );
  }
  return (
    <article
      aria-label={t("reader.verse_label", { surah: verse.surah, ayah: verse.ayah })}
      tabIndex={tabStop ? 0 : -1}
      data-highlighted={highlighted || undefined}
      data-selected={selected || undefined}
      data-playing={current || undefined}
      aria-current={current || undefined}
      className="group/verse flex gap-4 rounded-lg border-b border-border px-2 py-6 outline-none focus-visible:ring-2 focus-visible:ring-ring data-highlighted:animate-verse-highlight data-playing:bg-primary/5 data-playing:ring-1 data-playing:ring-primary/40 data-selected:bg-muted/60"
    >
      <div className="mt-2 flex shrink-0 flex-col items-center gap-1.5">
        <span
          aria-hidden
          className="flex size-7 items-center justify-center rounded-full border text-xs text-muted-foreground tabular-nums"
        >
          {verse.ayah}
        </span>
        {marks}
      </div>
      <div className="min-w-0 flex-1 space-y-3">
        <VerseArabic verse={verse} />
        {verse.translations.map((translation) => (
          <p
            key={translation.pack_id}
            lang={translation.lang ?? undefined}
            dir={packs[translation.pack_id]?.dir ?? "auto"}
            className="translation-text text-foreground/85"
          >
            {translation.text}
          </p>
        ))}
        {mode === "labeled" && actions}
      </div>
      {mode === "hover" && <div className="mt-1 shrink-0">{actions}</div>}
    </article>
  );
});

/**
 * A verse in reading mode: the number in an eight-pointed star, the Arabic
 * centred, and the translations below a divider, sliding open and shut. The
 * closed translations stay rendered (inert) so closing animates too.
 */
function VerseCard({
  verse,
  packs,
  expanded,
  onToggle,
  marks,
  actions,
  labeled,
  state,
}: {
  verse: VerseWithTranslations;
  packs: Readonly<Record<string, ShownPack>>;
  expanded: boolean;
  onToggle?: (verse: VerseWithTranslations) => void;
  marks: ReactNode;
  actions: ReactNode;
  labeled: boolean;
  state: { highlighted: boolean; selected: boolean; current: boolean; tabStop: boolean };
}) {
  const { t } = useTranslation();
  const regionId = useId();
  const many = verse.translations.length > 1;
  return (
    <div className="py-2">
      <article
        aria-label={t("reader.verse_label", { surah: verse.surah, ayah: verse.ayah })}
        tabIndex={state.tabStop ? 0 : -1}
        data-highlighted={state.highlighted || undefined}
        data-selected={state.selected || undefined}
        data-playing={state.current || undefined}
        data-expanded={expanded || undefined}
        aria-current={state.current || undefined}
        className="verse-card group/verse relative cursor-pointer rounded-2xl border bg-card px-5 pt-4 pb-2 text-card-foreground shadow-xs transition-[box-shadow,border-color,translate] duration-300 outline-none hover:-translate-y-px hover:border-primary/30 hover:shadow-md focus-visible:ring-2 focus-visible:ring-ring data-expanded:border-primary/25 data-expanded:shadow-md data-highlighted:animate-verse-highlight data-playing:ring-2 data-playing:ring-primary/40 data-selected:border-primary/40"
      >
        {!labeled && <div className="absolute end-2 top-2">{actions}</div>}
        <div className="flex items-center justify-center gap-2">
          <Medallion number={verse.ayah} turned={expanded} />
          {marks}
        </div>
        <div className="mt-2 text-center">
          <VerseArabic verse={verse} />
        </div>
        {verse.translations.length > 0 && (
          <>
            <div
              id={regionId}
              inert={!expanded}
              className={cn(
                "grid transition-[grid-template-rows] duration-300 ease-out",
                expanded ? "grid-rows-[1fr]" : "grid-rows-[0fr]",
              )}
            >
              <div className="min-h-0 overflow-hidden">
                <div
                  className={cn(
                    "mt-2 space-y-4 border-t border-dashed border-primary/25 pt-4 pb-1 transition-[opacity,translate] duration-300 ease-out",
                    expanded ? "translate-y-0 opacity-100" : "-translate-y-2 opacity-0",
                  )}
                >
                  {verse.translations.map((translation) => (
                    <div key={translation.pack_id} className="space-y-1">
                      {many && (
                        <p className="text-xs font-medium tracking-wide text-primary uppercase">
                          {packs[translation.pack_id]?.title}
                        </p>
                      )}
                      <p
                        lang={translation.lang ?? undefined}
                        dir={packs[translation.pack_id]?.dir ?? "auto"}
                        className="translation-text"
                      >
                        {translation.text}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            </div>
            <button
              type="button"
              aria-expanded={expanded}
              aria-controls={regionId}
              tabIndex={-1}
              onClick={() => {
                onToggle?.(verse);
              }}
              className="mx-auto mt-1 flex items-center gap-1 rounded-full px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-primary/10 hover:text-primary"
            >
              {expanded ? t("reader.hide_translation") : t("reader.show_translation")}
              <ChevronDownIcon
                aria-hidden
                className={cn(
                  "size-3.5 transition-transform duration-300",
                  expanded && "rotate-180",
                )}
              />
            </button>
          </>
        )}
        {labeled && actions}
      </article>
    </div>
  );
}

/** A verse number in an eight-pointed star, which turns when its card opens. */
function Medallion({ number, turned }: { number: number; turned: boolean }) {
  const square = "absolute inset-1.5 rounded-[3px] border border-primary/40 bg-primary/10";
  return (
    <span
      aria-hidden
      className="relative flex size-10 items-center justify-center text-xs font-semibold text-primary tabular-nums"
    >
      <span
        className={cn(
          "absolute inset-0 transition-transform duration-500 ease-out",
          turned && "rotate-45",
        )}
      >
        <span className={square} />
        <span className={cn(square, "rotate-45")} />
      </span>
      <span className="relative">{number}</span>
    </span>
  );
}
