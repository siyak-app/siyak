import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { verseIdOf } from "@/features/tafsir/tafsirRefs";
import { MorphologyRequired } from "@/features/word/MorphologyRequired";
import { MorphologySource } from "@/features/word/MorphologySource";
import type { RelatedVerse } from "@/lib/bindings";
import {
  useMorphologyPack,
  useRelatedByMeaning,
  useRelatedByRoots,
  useSemanticStatus,
  useSurahs,
} from "@/lib/queries";
import type { VersePosition } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { RelatedVerseList } from "./RelatedVerseList";
import { SemanticRequired } from "./SemanticRequired";

/**
 * The Related tab (spec §7.6): verses close in meaning to the selected one and
 * verses sharing its rare roots, in two sections, each with its reason. Each
 * section needs its own pack and says so when it is missing.
 */
export function RelatedPanel({ verse }: { verse: VersePosition | null }) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  if (!verse) return <Hint>{t("related.select_verse")}</Hint>;
  if (!surahs.data) return <Hint>{t("related.loading")}</Hint>;
  const verseId = verseIdOf(surahs.data, verse);
  return (
    <div className="flex h-full min-h-0 flex-col gap-5 overflow-y-auto pt-2">
      <ByMeaning verseId={verseId} />
      <ByRoots verseId={verseId} />
    </div>
  );
}

function Hint({ children }: { children: string }) {
  return <p className="p-1 text-sm text-muted-foreground">{children}</p>;
}

function Section({ title, hint, children }: { title: string; hint: string; children: ReactNode }) {
  return (
    <section className="grid gap-2">
      <header className="grid gap-0.5 px-1">
        <h3 className="text-sm font-medium">{title}</h3>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </header>
      {children}
    </section>
  );
}

/** The first translation the reader shows: related verses read the same way. */
function useReaderTranslation() {
  return useUiStore((state) => state.preferences["reader.translations"][0] ?? null);
}

function ByMeaning({ verseId }: { verseId: number }) {
  const { t } = useTranslation();
  const status = useSemanticStatus();
  // Comparing verse vectors needs only the vector pack, not the model.
  const hasVectors =
    status.data !== undefined &&
    status.data.state !== "no_vector_pack" &&
    status.data.state !== "invalid";
  const related = useRelatedByMeaning(verseId, useReaderTranslation(), hasVectors);
  let body: ReactNode;
  if (status.isError) body = <ErrorMessage error={status.error} />;
  else if (!status.data) body = <Hint>{t("related.loading")}</Hint>;
  else if (status.data.state === "no_vector_pack" || status.data.state === "invalid")
    body = <SemanticRequired status={status.data} />;
  else body = <Results query={related} />;
  return (
    <Section title={t("related.by_meaning")} hint={t("related.by_meaning_hint")}>
      {body}
    </Section>
  );
}

function ByRoots({ verseId }: { verseId: number }) {
  const { t } = useTranslation();
  const pack = useMorphologyPack();
  const related = useRelatedByRoots(verseId, useReaderTranslation(), Boolean(pack.data));
  let body: ReactNode;
  if (pack.isError) body = <ErrorMessage error={pack.error} />;
  else if (pack.isPending) body = <Hint>{t("related.loading")}</Hint>;
  else if (pack.data === null) body = <MorphologyRequired />;
  else
    body = (
      <>
        <Results query={related} />
        <MorphologySource />
      </>
    );
  return (
    <Section title={t("related.by_roots")} hint={t("related.by_roots_hint")}>
      {body}
    </Section>
  );
}

function Results({
  query,
}: {
  query: { data?: RelatedVerse[]; isError: boolean; error: Error | null };
}) {
  const { t } = useTranslation();
  if (query.isError) return <ErrorMessage error={query.error} />;
  if (!query.data) return <Hint>{t("related.loading")}</Hint>;
  if (query.data.length === 0) return <Hint>{t("related.none")}</Hint>;
  return <RelatedVerseList verses={query.data} compact />;
}
