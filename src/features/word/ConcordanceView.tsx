import { Link } from "@tanstack/react-router";
import {
  AddToCollectionButton,
  AddVerseButton,
} from "@/features/collections/AddToCollectionButton";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Term } from "@/components/Term";
import { Button } from "@/components/ui/button";
import { surahName } from "@/features/reader/surahName";
import { verseNumber, verseReference } from "@/features/reader/verseCopy";
import type { ConcordanceTarget, ConcordanceVerse } from "@/lib/bindings";
import { useConcordance, useSurahs } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { LemmaText } from "./LemmaText";
import { MorphologySource } from "./MorphologySource";
import { SurahDistribution } from "./SurahDistribution";

/** Verses added by each "Show more". */
const PAGE = 50;

/** The concordance screen: `/concordance/:kind/:id`. */
export function ConcordanceView({ target }: { target: ConcordanceTarget }) {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl p-6">
        <ConcordanceContent
          key={`${target.kind}-${String(target.id)}`}
          target={target}
          heading="h1"
        />
      </div>
    </div>
  );
}

/**
 * Every occurrence of a root or lemma (spec §7.3, §13.3): the root, its
 * dictionary forms to narrow down to, the distribution over the surahs and the
 * verses. Also the result of a root search, which is why it takes its heading level.
 */
export function ConcordanceContent({
  target,
  heading: Heading,
}: {
  target: ConcordanceTarget;
  heading: "h1" | "h2";
}) {
  const { t } = useTranslation();
  // The first translation the reader shows: the verses read the same way here.
  const translationId = useUiStore((state) => state.preferences["reader.translations"][0] ?? null);
  const [limit, setLimit] = useState(PAGE);
  const concordance = useConcordance({ target, translationId, limit });
  if (concordance.isError) return <ErrorMessage error={concordance.error} />;
  if (!concordance.data)
    return <p className="text-sm text-muted-foreground">{t("word.loading")}</p>;
  const { root, lemma, lemmas, occurrences, verse_count: verseCount } = concordance.data;
  return (
    <div className="grid gap-4">
      <header className="grid gap-1">
        <p className="text-xs text-muted-foreground first-letter:uppercase">
          <Term name="concordance" />
        </p>
        <Heading dir="rtl" lang="ar" className="text-start font-quran text-3xl leading-loose">
          {lemma ? <LemmaText lemma={lemma.lemma} /> : root?.root}
        </Heading>
        <p role="status" className="text-sm text-muted-foreground">
          {t("concordance.summary", { count: occurrences, verses: verseCount })}
        </p>
        <div>
          <AddToCollectionButton
            item={
              lemma
                ? { kind: "lemma", lemma: lemma.lemma }
                : { kind: "root", root: root?.root ?? "" }
            }
          />
        </div>
      </header>
      {(root !== null || lemmas.length > 1) && (
        <nav aria-label={t("concordance.narrow")} className="grid gap-1.5">
          <p className="text-xs font-medium first-letter:uppercase">
            <Term name="lemma" />
          </p>
          <ul className="flex flex-wrap gap-1">
            {root && (
              <li>
                <Button asChild size="xs" variant={lemma ? "ghost" : "secondary"}>
                  <Link
                    to="/concordance/$kind/$id"
                    params={{ kind: "root", id: root.id }}
                    aria-current={lemma ? undefined : "page"}
                  >
                    {t("concordance.all_forms", { count: root.occurrences })}
                  </Link>
                </Button>
              </li>
            )}
            {lemmas.map((item) => (
              <li key={item.id}>
                <Button asChild size="xs" variant={item.id === lemma?.id ? "secondary" : "ghost"}>
                  <Link
                    to="/concordance/$kind/$id"
                    params={{ kind: "lemma", id: item.id }}
                    aria-current={item.id === lemma?.id ? "page" : undefined}
                  >
                    <LemmaText lemma={item.lemma} />
                    <span className="text-muted-foreground tabular-nums">{item.occurrences}</span>
                  </Link>
                </Button>
              </li>
            ))}
          </ul>
        </nav>
      )}
      <SurahDistribution counts={concordance.data.by_surah} />
      <ul className="grid gap-2">
        {concordance.data.verses.map((verse) => (
          <li key={`${String(verse.surah)}:${String(verse.ayah)}`}>
            <VerseHit verse={verse} />
          </li>
        ))}
      </ul>
      {verseCount > concordance.data.verses.length && (
        <Button
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() => {
            setLimit(limit + PAGE);
          }}
        >
          {t("concordance.show_more")}
        </Button>
      )}
      <MorphologySource />
    </div>
  );
}

/** One verse with the listed words marked: opens it in the reader. */
function VerseHit({ verse }: { verse: ConcordanceVerse }) {
  const { i18n } = useTranslation();
  const surahs = useSurahs();
  const surah = surahs.data?.find((item) => item.number === verse.surah);
  return (
    <div className="relative">
      <AddVerseButton verse={verse} className="absolute end-2 top-2" />
      <Link
        to="/read/$surah/$ayah"
        params={{ surah: verse.surah, ayah: verse.ayah }}
        className="grid gap-1 rounded-lg border p-3 transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
      >
        <span className="text-xs text-muted-foreground">
          {surah
            ? verseReference(surahName(surah, i18n.resolvedLanguage), verse)
            : verseNumber(verse)}
        </span>
        <span dir="rtl" lang="ar" className="font-quran text-xl leading-loose">
          {verse.words.map((word, index) => (
            // The words of a verse never change order.
            <span key={index}>
              {index > 0 && " "}
              {word.matched ? (
                <mark className="rounded-sm bg-primary/15 text-foreground">{word.text_ar}</mark>
              ) : (
                word.text_ar
              )}
            </span>
          ))}
        </span>
        {verse.translation && (
          <span lang={verse.translation.lang ?? undefined} dir="auto" className="translation-text">
            {verse.translation.text}
          </span>
        )}
      </Link>
    </div>
  );
}
