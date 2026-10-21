import { memo } from "react";
import { useTranslation } from "react-i18next";

import type { TextDirection, VerseWithTranslations } from "@/lib/bindings";

/**
 * One verse: Arabic text followed by the selected translations, each in its
 * pack's language and direction. Memoized: scrolling re-renders only the
 * verses entering the virtualized window.
 */
export const VerseItem = memo(function VerseItem({
  verse,
  directions,
  highlighted = false,
}: {
  verse: VerseWithTranslations;
  /** Writing direction per translation pack id. */
  directions: Readonly<Record<string, TextDirection>>;
  /** Briefly marks the verse a deep link opened. */
  highlighted?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <article
      aria-label={t("reader.verse_label", { surah: verse.surah, ayah: verse.ayah })}
      data-highlighted={highlighted || undefined}
      className="flex gap-4 rounded-lg border-b border-border px-2 py-6 data-highlighted:animate-verse-highlight"
    >
      <span
        aria-hidden
        className="mt-2 flex size-7 shrink-0 items-center justify-center rounded-full border text-xs text-muted-foreground tabular-nums"
      >
        {verse.ayah}
      </span>
      <div className="min-w-0 flex-1 space-y-3">
        <p dir="rtl" lang="ar" className="arabic-text">
          {verse.text_uthmani}
        </p>
        {verse.translations.map((translation) => (
          <p
            key={translation.pack_id}
            lang={translation.lang ?? undefined}
            dir={directions[translation.pack_id] ?? "auto"}
            className="translation-text text-foreground/85"
          >
            {translation.text}
          </p>
        ))}
      </div>
    </article>
  );
});
