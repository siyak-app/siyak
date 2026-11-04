import { BookmarkIcon, NotebookPenIcon } from "lucide-react";
import { memo } from "react";
import { useTranslation } from "react-i18next";

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
 * pack's language and direction, and the verse's actions. Memoized:
 * scrolling re-renders only the verses entering the virtualized window.
 */
export const VerseItem = memo(function VerseItem({
  verse,
  packs,
  surahName,
  highlighted = false,
  tabStop = false,
  onCompare,
  bookmarked = false,
  noted = false,
  onBookmark,
}: {
  verse: VerseWithTranslations;
  /** The shown translation packs by id. */
  packs: Readonly<Record<string, ShownPack>>;
  /** The surah's name in the interface language. */
  surahName: string;
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
}) {
  const { t } = useTranslation();
  const mode = useUiStore((state) => state.preferences["reader.actions"]);
  const selected = useUiStore(
    (state) =>
      state.selectedVerse?.surah === verse.surah && state.selectedVerse.ayah === verse.ayah,
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
    />
  );
  return (
    <article
      aria-label={t("reader.verse_label", { surah: verse.surah, ayah: verse.ayah })}
      tabIndex={tabStop ? 0 : -1}
      data-highlighted={highlighted || undefined}
      data-selected={selected || undefined}
      className="group/verse flex gap-4 rounded-lg border-b border-border px-2 py-6 outline-none focus-visible:ring-2 focus-visible:ring-ring data-highlighted:animate-verse-highlight data-selected:bg-muted/60"
    >
      <div className="mt-2 flex shrink-0 flex-col items-center gap-1.5">
        <span
          aria-hidden
          className="flex size-7 items-center justify-center rounded-full border text-xs text-muted-foreground tabular-nums"
        >
          {verse.ayah}
        </span>
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
