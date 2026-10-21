import { useTranslation } from "react-i18next";

import type { VerseWithTranslations } from "@/lib/bindings";

/** One verse: Arabic text followed by the selected translations. */
export function VerseItem({ verse }: { verse: VerseWithTranslations }) {
  const { t } = useTranslation();
  return (
    <article
      aria-label={t("reader.verse_label", { surah: verse.surah, ayah: verse.ayah })}
      className="flex gap-4 border-b border-border py-6 last:border-b-0"
    >
      <span className="mt-2 flex size-7 shrink-0 items-center justify-center rounded-full border text-xs text-muted-foreground tabular-nums">
        {verse.ayah}
      </span>
      <div className="min-w-0 flex-1 space-y-3">
        <p dir="rtl" lang="ar" className="font-quran text-[1.75rem] leading-[2.2]">
          {verse.text_uthmani}
        </p>
        {verse.translations.map((translation) => (
          <p
            key={translation.pack_id}
            lang={translation.lang ?? undefined}
            dir="auto"
            className="leading-relaxed text-foreground/80"
          >
            {translation.text}
          </p>
        ))}
      </div>
    </article>
  );
}
