import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Term } from "@/components/Term";
import { Button } from "@/components/ui/button";
import { useWord } from "@/lib/queries";

import { LemmaText } from "./LemmaText";
import { mainSegment, posLabel } from "./morphologyLabels";
import { MorphologySource } from "./MorphologySource";

/**
 * The small word card (`word.click` = `card`, spec §13.2): the word's meaning
 * if the pack has one, its root and dictionary form with how often they occur,
 * and the way to the full analysis.
 */
export function WordCard({ wordId, onDetails }: { wordId: number; onDetails: () => void }) {
  const { t } = useTranslation();
  const word = useWord(wordId);
  if (word.isError) return <ErrorMessage error={word.error} />;
  if (!word.data) return <p className="text-sm text-muted-foreground">{t("word.loading")}</p>;
  const main = mainSegment(word.data);
  return (
    <div className="grid gap-2">
      <p dir="rtl" lang="ar" className="font-quran text-2xl leading-loose">
        {word.data.text_ar}
      </p>
      {word.data.gloss_en !== null && <p className="text-sm">{word.data.gloss_en}</p>}
      {main && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
          <dt className="text-muted-foreground first-letter:uppercase">
            <Term name="part_of_speech" />
          </dt>
          <dd>{posLabel(t, main.pos)}</dd>
          {main.root && (
            <>
              <dt className="text-muted-foreground first-letter:uppercase">
                <Term name="root" />
              </dt>
              <dd>
                <Link
                  to="/concordance/$kind/$id"
                  params={{ kind: "root", id: main.root.id }}
                  className="underline underline-offset-2"
                >
                  <span dir="rtl" lang="ar">
                    {main.root.root}
                  </span>
                  {` (${String(main.root.occurrences)})`}
                </Link>
              </dd>
            </>
          )}
          {main.lemma && (
            <>
              <dt className="text-muted-foreground first-letter:uppercase">
                <Term name="lemma" />
              </dt>
              <dd>
                <Link
                  to="/concordance/$kind/$id"
                  params={{ kind: "lemma", id: main.lemma.id }}
                  className="underline underline-offset-2"
                >
                  <LemmaText lemma={main.lemma.lemma} />
                  {` (${String(main.lemma.occurrences)})`}
                </Link>
              </dd>
            </>
          )}
        </dl>
      )}
      <Button variant="outline" size="sm" className="justify-self-start" onClick={onDetails}>
        {t("word.details")}
      </Button>
      <MorphologySource />
    </div>
  );
}
