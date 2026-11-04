import type { VerseWithTranslations } from "@/lib/bindings";
import { useTranslations } from "@/lib/queries";

/** A verse's Arabic text and its translations, each in its language and direction. */
export function VerseText({ verse }: { verse: VerseWithTranslations }) {
  const packs = useTranslations();
  return (
    <div className="grid gap-2">
      <p dir="rtl" lang="ar" className="font-quran text-xl leading-loose">
        {verse.text_uthmani}
      </p>
      {verse.translations.map((translation) => (
        <p
          key={translation.pack_id}
          lang={translation.lang ?? undefined}
          dir={packs.data?.find((pack) => pack.id === translation.pack_id)?.dir ?? "auto"}
          className="translation-text text-foreground/85"
        >
          {translation.text}
        </p>
      ))}
    </div>
  );
}
