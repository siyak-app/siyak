import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { AddVerseButton } from "@/features/collections/AddToCollectionButton";
import { surahName } from "@/features/reader/surahName";
import { verseNumber, verseReference } from "@/features/reader/verseCopy";
import { LemmaText } from "@/features/word/LemmaText";
import type { RelatedVerse } from "@/lib/bindings";
import { useSurahs } from "@/lib/queries";

/**
 * Verses found by meaning or by shared roots (spec §7.4, §7.6). Each opens in
 * the reader; shared roots and names are listed under it as the reason and
 * lead to their concordance. `compact` (the side panel) shows the translation
 * alone when there is one.
 */
export function RelatedVerseList({
  verses,
  compact = false,
}: {
  verses: RelatedVerse[];
  compact?: boolean;
}) {
  return (
    <ul className="grid gap-2">
      {verses.map((verse) => (
        <li key={`${String(verse.surah)}:${String(verse.ayah)}`}>
          <RelatedVerseItem verse={verse} compact={compact} />
        </li>
      ))}
    </ul>
  );
}

function RelatedVerseItem({ verse, compact }: { verse: RelatedVerse; compact: boolean }) {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  const surah = surahs.data?.find((item) => item.number === verse.surah);
  const showArabic = !compact || verse.translation === null;
  return (
    <div className="grid gap-1.5 rounded-lg border p-3">
      <div className="relative">
        <AddVerseButton verse={verse} className="absolute end-0 top-0" />
        <Link
          to="/read/$surah/$ayah"
          params={{ surah: verse.surah, ayah: verse.ayah }}
          className="grid gap-1 rounded-md pe-8 transition-colors hover:text-primary focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        >
          <span className="text-xs text-muted-foreground">
            {surah
              ? verseReference(surahName(surah, i18n.resolvedLanguage), verse)
              : verseNumber(verse)}
          </span>
          {showArabic && (
            <span
              dir="rtl"
              lang="ar"
              className={
                compact ? "font-quran text-lg leading-loose" : "font-quran text-xl leading-loose"
              }
            >
              {verse.text_ar}
            </span>
          )}
          {verse.translation && (
            <span
              lang={verse.translation.lang ?? undefined}
              dir="auto"
              className={compact ? "text-sm" : "translation-text"}
            >
              {verse.translation.text}
            </span>
          )}
        </Link>
      </div>
      {verse.shared.length > 0 && (
        <p className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-xs text-muted-foreground">
          <span>{t("related.shared")}</span>
          {verse.shared.map((term) => (
            <Link
              key={`${term.target.kind}-${String(term.target.id)}`}
              to="/concordance/$kind/$id"
              params={{ kind: term.target.kind, id: term.target.id }}
              dir="rtl"
              lang="ar"
              className="rounded-sm font-quran text-base text-foreground underline-offset-4 hover:underline focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            >
              {term.target.kind === "lemma" ? <LemmaText lemma={term.text} /> : term.text}
            </Link>
          ))}
        </p>
      )}
    </div>
  );
}
