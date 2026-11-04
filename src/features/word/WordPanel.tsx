import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Term } from "@/components/Term";
import { Button } from "@/components/ui/button";
import { verseIdOf } from "@/features/tafsir/tafsirRefs";
import type { WordDetail, WordSegment } from "@/lib/bindings";
import { useMorphologyPack, useSurahs, useVerseWords, useWord } from "@/lib/queries";
import type { VersePosition } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { LemmaText } from "./LemmaText";
import { featureRows, posLabel, wordLemmas, wordRoots } from "./morphologyLabels";
import { MorphologyRequired } from "./MorphologyRequired";
import { MorphologySource } from "./MorphologySource";

/**
 * The Word tab (spec §7.5): the words of the selected verse and the analysis
 * of the chosen one. The list makes every word reachable by keyboard: `w` on a
 * verse opens this tab.
 */
export function WordPanel({ verse }: { verse: VersePosition | null }) {
  const { t } = useTranslation();
  const pack = useMorphologyPack();
  if (pack.isError) return <ErrorMessage error={pack.error} />;
  if (pack.isPending) return <Hint>{t("word.loading")}</Hint>;
  if (pack.data === null) {
    return (
      <div className="pt-2">
        <MorphologyRequired />
      </div>
    );
  }
  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto pt-2">
      {verse ? <VerseWords verse={verse} /> : <Hint>{t("word.select_word")}</Hint>}
      <MorphologySource />
    </div>
  );
}

function Hint({ children }: { children: string }) {
  return <p className="p-1 text-sm text-muted-foreground">{children}</p>;
}

function VerseWords({ verse }: { verse: VersePosition }) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  const verseId = surahs.data ? verseIdOf(surahs.data, verse) : null;
  const words = useVerseWords(verseId);
  const selected = useUiStore((state) => state.selectedWord);
  const selectWord = useUiStore((state) => state.selectWord);
  if (words.isError) return <ErrorMessage error={words.error} />;
  if (!words.data) return <Hint>{t("word.loading")}</Hint>;
  const current = words.data.some((word) => word.id === selected) ? selected : null;
  return (
    <>
      <div
        role="group"
        aria-label={t("word.verse_words")}
        dir="rtl"
        lang="ar"
        className="flex flex-wrap gap-1"
      >
        {words.data.map((word) => (
          <Button
            key={word.id}
            variant={word.id === current ? "secondary" : "ghost"}
            size="sm"
            aria-pressed={word.id === current}
            className="h-auto font-quran text-lg leading-loose"
            onClick={() => {
              selectWord(verse, word.id);
            }}
          >
            {word.text_ar}
          </Button>
        ))}
      </div>
      {current === null ? <Hint>{t("word.select_word")}</Hint> : <Analysis wordId={current} />}
    </>
  );
}

function Analysis({ wordId }: { wordId: number }) {
  const { t } = useTranslation();
  const word = useWord(wordId);
  if (word.isError) return <ErrorMessage error={word.error} />;
  if (!word.data) return <Hint>{t("word.loading")}</Hint>;
  return (
    <section aria-label={t("word.analysis")} className="grid gap-3">
      <header className="grid gap-1">
        <p dir="rtl" lang="ar" className="font-quran text-3xl leading-loose">
          {word.data.text_ar}
        </p>
        <p className="text-xs text-muted-foreground tabular-nums">
          {`${String(word.data.surah)}:${String(word.data.ayah)}:${String(word.data.position)}`}
        </p>
        {word.data.translit !== null && <p className="text-sm">{word.data.translit}</p>}
        {word.data.gloss_en !== null && <p className="text-sm">{word.data.gloss_en}</p>}
      </header>
      <ul className="grid gap-2">
        {word.data.segments.map((segment, index) => (
          // The segments of a word never change order.
          <li key={index}>
            <Segment segment={segment} />
          </li>
        ))}
      </ul>
      <Occurrences word={word.data} />
    </section>
  );
}

function Segment({ segment }: { segment: WordSegment }) {
  const { t } = useTranslation();
  const rows = featureRows(t, segment.features);
  return (
    <div className="grid gap-1.5 rounded-lg border p-2.5 text-sm">
      <div className="flex items-baseline justify-between gap-2">
        <span>
          <span className="font-medium">{posLabel(t, segment.pos)}</span>
          <span className="text-muted-foreground"> · {t(`word.kind.${segment.kind}`)}</span>
        </span>
        <span dir="rtl" lang="ar" className="font-quran text-xl leading-loose">
          {segment.text_ar}
        </span>
      </div>
      {(segment.root !== null || segment.lemma !== null || rows.length > 0) && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs">
          {segment.root && (
            <>
              <dt className="text-muted-foreground first-letter:uppercase">
                <Term name="root" />
              </dt>
              <dd className="text-sm">
                <span dir="rtl" lang="ar">
                  {segment.root.root}
                </span>
              </dd>
            </>
          )}
          {segment.lemma && (
            <>
              <dt className="text-muted-foreground first-letter:uppercase">
                <Term name="lemma" />
              </dt>
              <dd className="text-sm">
                <LemmaText lemma={segment.lemma.lemma} />
              </dd>
            </>
          )}
          {rows.map((row) => (
            <div key={row.key} className="contents">
              <dt className="text-muted-foreground">{row.label}</dt>
              <dd>{row.value}</dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

/** The links to the concordance. */
function Occurrences({ word }: { word: WordDetail }) {
  const { t } = useTranslation();
  const roots = wordRoots(word);
  const lemmas = wordLemmas(word);
  if (roots.length === 0 && lemmas.length === 0) return null;
  return (
    <nav aria-label={t("word.occurrences")} className="grid justify-items-start gap-1 text-sm">
      {roots.map((root) => (
        <Link
          key={root.id}
          to="/concordance/$kind/$id"
          params={{ kind: "root", id: root.id }}
          className="underline underline-offset-2"
        >
          {t("word.root_occurrences", { count: root.occurrences })}
          {roots.length > 1 && ` · ${root.root}`}
        </Link>
      ))}
      {lemmas.map((lemma) => (
        <Link
          key={lemma.id}
          to="/concordance/$kind/$id"
          params={{ kind: "lemma", id: lemma.id }}
          className="underline underline-offset-2"
        >
          {t("word.lemma_occurrences", { count: lemma.occurrences })}
          {lemmas.length > 1 && ` · ${lemma.lemma}`}
        </Link>
      ))}
    </nav>
  );
}
