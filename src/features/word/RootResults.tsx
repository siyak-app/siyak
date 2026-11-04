import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Term } from "@/components/Term";
import { Button } from "@/components/ui/button";
import type { RootSearch } from "@/lib/bindings";
import { useMorphologyPack, useSearchRoots } from "@/lib/queries";

import { ConcordanceContent } from "./ConcordanceView";
import { LemmaText } from "./LemmaText";
import { singleTarget } from "./morphologyLabels";
import { MorphologyRequired } from "./MorphologyRequired";

/** Matches shown in the All group (spec §7.1: the first 3–5). */
const ALL_LIMIT = 5;

/** Root mode (spec §7.3): the concordance when the query means one thing, else a choice. */
export function RootResults({ q }: { q: string }) {
  const { t } = useTranslation();
  const pack = useMorphologyPack();
  const found = useSearchRoots(q, pack.data != null);
  if (pack.data === null) return <MorphologyRequired />;
  if (found.isError) return <ErrorMessage error={found.error} />;
  if (!found.data) return <p className="text-sm text-muted-foreground">{t("search.searching")}</p>;
  if (found.data.roots.length === 0 && found.data.lemmas.length === 0) {
    return <p className="text-sm text-muted-foreground">{t("search.no_root_results")}</p>;
  }
  const target = singleTarget(found.data);
  if (target) {
    return (
      <ConcordanceContent
        key={`${target.kind}-${String(target.id)}`}
        target={target}
        heading="h2"
      />
    );
  }
  return <Matches found={found.data} />;
}

/**
 * The root and dictionary form group of All mode (spec §7.1), above the text
 * matches. Stays out of the way when an Arabic query is neither.
 */
export function RootGroup({ q, emptyNote = false }: { q: string; emptyNote?: boolean }) {
  const { t } = useTranslation();
  const pack = useMorphologyPack();
  const found = useSearchRoots(q, pack.data != null);
  if (pack.data === null) return <MorphologyRequired />;
  if (found.isError) return <ErrorMessage error={found.error} />;
  if (!found.data) return null;
  const count = found.data.roots.length + found.data.lemmas.length;
  if (count === 0) {
    return emptyNote ? (
      <p className="text-sm text-muted-foreground">{t("search.no_root_results")}</p>
    ) : null;
  }
  const roots = found.data.roots.slice(0, ALL_LIMIT);
  const lemmas = found.data.lemmas.slice(0, ALL_LIMIT - roots.length);
  return (
    <section className="grid gap-3">
      <h2 className="text-sm font-medium">{t("search.group_roots")}</h2>
      <Matches found={{ roots, lemmas }} />
      {count > ALL_LIMIT && (
        <Button asChild variant="outline" size="sm" className="justify-self-start">
          <Link to="/search" search={{ q, mode: "root" }}>
            {t("search.see_all", { count })}
          </Link>
        </Button>
      )}
    </section>
  );
}

function Matches({ found }: { found: RootSearch }) {
  const { t } = useTranslation();
  const linkClass =
    "flex items-baseline justify-between gap-3 rounded-lg border p-3 transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none";
  return (
    <ul className="grid gap-2">
      {found.roots.map((root) => (
        <li key={`root-${String(root.id)}`}>
          <Link
            to="/concordance/$kind/$id"
            params={{ kind: "root", id: root.id }}
            className={linkClass}
          >
            <span className="text-xs text-muted-foreground first-letter:uppercase">
              <Term name="root" />
            </span>
            <span dir="rtl" lang="ar" className="font-quran text-xl leading-loose">
              {root.root}
            </span>
            <span className="ms-auto text-sm text-muted-foreground tabular-nums">
              {t("concordance.occurrences", { count: root.occurrences })}
            </span>
          </Link>
        </li>
      ))}
      {found.lemmas.map((lemma) => (
        <li key={`lemma-${String(lemma.id)}`}>
          <Link
            to="/concordance/$kind/$id"
            params={{ kind: "lemma", id: lemma.id }}
            className={linkClass}
          >
            <span className="text-xs text-muted-foreground first-letter:uppercase">
              <Term name="lemma" />
            </span>
            <span className="font-quran text-xl leading-loose">
              <LemmaText lemma={lemma.lemma} />
            </span>
            <span className="ms-auto text-sm text-muted-foreground tabular-nums">
              {t("concordance.occurrences", { count: lemma.occurrences })}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}
