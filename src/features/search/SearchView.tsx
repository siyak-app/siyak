import { Link, useNavigate } from "@tanstack/react-router";
import { AddVerseButton } from "@/features/collections/AddToCollectionButton";
import { ArrowRightIcon } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { surahName } from "@/features/reader/surahName";
import { translationTitle } from "@/features/reader/translationTitle";
import { verseNumber, verseReference } from "@/features/reader/verseCopy";
import { RelatedVerseList } from "@/features/related/RelatedVerseList";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import { SemanticRequired } from "@/features/related/SemanticRequired";
import { RadioField } from "@/features/settings/fields";
import { RootGroup, RootResults } from "@/features/word/RootResults";
import type {
  RevelationType,
  SearchFilters,
  SearchHit,
  SearchScope,
  TranslationInfo,
} from "@/lib/bindings";
import {
  useClassifyQuery,
  useSearchMeaning,
  useSearchText,
  useSemanticStatus,
  useSurahs,
  useTranslations,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { SEARCH_MODES, type SearchMode, type SearchParams } from "./searchParams";

/** Results shown per group in All mode (spec §7.1: the first 3–5). */
const ALL_LIMIT = 5;
/** Results added by each "Show more" in Text mode. */
const PAGE = 50;
/** Verses shown in Meaning mode, and added by each "Show more", up to the core's limit. */
const MEANING_PAGE = 20;
const MEANING_MAX = 50;

const NO_FILTERS: SearchFilters = { surah_from: null, surah_to: null, revelation: null };

/**
 * The search screen (spec §7.1, §7.2). All shows the first matches of the
 * group the query belongs to, Text the full list with filters, Root the
 * concordance (spec §7.3), Meaning the verses closest in meaning (spec §7.4).
 * The Arabic text is searched for Arabic queries, the translations for
 * anything else; in All, an Arabic query or a Latin-script root also looks
 * for a root or dictionary form, and other text for verses similar in meaning.
 */
export function SearchView({ q, mode }: SearchParams) {
  const { t } = useTranslation();
  const chips = useUiStore((state) => state.preferences["search.mode_chips"]);
  const classified = useClassifyQuery(q);

  let body = null;
  if (classified.isError) {
    body = <ErrorMessage error={classified.error} />;
  } else if (classified.data) {
    const result = classified.data;
    if (result.kind === "empty") {
      body = <Note>{t("search.empty")}</Note>;
    } else if (result.kind === "reference") {
      const verse = { surah: result.surah, ayah: result.ayah ?? 1 };
      body = (
        <Button asChild variant="outline" className="justify-self-start">
          <Link to="/read/$surah/$ayah" params={verse}>
            <ArrowRightIcon aria-hidden />
            {t("search.go_to", { reference: verseNumber(verse) })}
          </Link>
        </Button>
      );
    } else if (result.kind === "invalid_reference") {
      body = <Note>{t(`command_palette.errors.${result.reason}`)}</Note>;
    } else if (mode === "root") {
      body = <RootResults q={q} />;
    } else if (mode === "meaning") {
      body = <MeaningResults key={q} q={q} />;
    } else {
      const scope: SearchScope = result.kind === "arabic" ? "arabic" : "translations";
      if (mode === "text") {
        body = <TextResults key={`${q}-${scope}`} q={q} scope={scope} />;
      } else if (result.kind === "latin_root") {
        body = <RootGroup q={q} emptyNote />;
      } else if (result.kind === "arabic") {
        body = (
          <>
            <RootGroup q={q} />
            <AllResults q={q} scope={scope} />
          </>
        );
      } else {
        body = (
          <>
            <AllResults q={q} scope={scope} />
            <MeaningGroup q={q} />
          </>
        );
      }
    }
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-6">
        <header className="grid gap-2">
          <h1 className="text-lg font-semibold" dir="auto">
            {q === "" ? t("search.title") : t("search.results_for", { query: q })}
          </h1>
          {chips && <ModeChips q={q} mode={mode} />}
        </header>
        {body}
      </div>
    </div>
  );
}

function Note({ children }: { children: string }) {
  return <p className="text-sm text-muted-foreground">{children}</p>;
}

function ModeChips({ q, mode }: SearchParams) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <div role="group" aria-label={t("search.modes_label")} className="flex gap-1">
      {SEARCH_MODES.map((value: SearchMode) => (
        <Button
          key={value}
          size="xs"
          variant={value === mode ? "secondary" : "ghost"}
          aria-pressed={value === mode}
          onClick={() => {
            void navigate({ to: "/search", search: { q, mode: value } });
          }}
        >
          {t(`search.mode_${value}`)}
        </Button>
      ))}
    </div>
  );
}

/** The first matches, with a link to the full list. */
function AllResults({ q, scope }: { q: string; scope: SearchScope }) {
  const { t } = useTranslation();
  const results = useSearchText({
    query: q,
    scope,
    packIds: [],
    filters: NO_FILTERS,
    limit: ALL_LIMIT,
  });
  if (results.isError) return <ErrorMessage error={results.error} />;
  if (!results.data) return <Note>{t("search.searching")}</Note>;
  if (results.data.total === 0) return <Note>{t("search.no_results")}</Note>;
  return (
    <section className="grid gap-3">
      <h2 className="text-sm font-medium">
        {scope === "arabic" ? t("search.group_arabic") : t("search.group_translations")}
      </h2>
      <HitList hits={results.data.hits} />
      {results.data.total > results.data.hits.length && (
        <Button asChild variant="outline" size="sm" className="justify-self-start">
          <Link to="/search" search={{ q, mode: "text" }}>
            {t("search.see_all", { count: results.data.total })}
          </Link>
        </Button>
      )}
    </section>
  );
}

/** The first translation the reader shows: found verses read the same way. */
function useReaderTranslation() {
  return useUiStore((state) => state.preferences["reader.translations"][0] ?? null);
}

/** The first verses similar in meaning, with a link to the longer list (spec §7.1). */
function MeaningGroup({ q }: { q: string }) {
  const { t } = useTranslation();
  const status = useSemanticStatus();
  const ready = status.data?.state === "ready";
  const results = useSearchMeaning(
    { query: q, translationId: useReaderTranslation(), limit: ALL_LIMIT },
    ready,
  );
  let body;
  if (status.isError) body = <ErrorMessage error={status.error} />;
  else if (!status.data) body = <Note>{t("search.searching")}</Note>;
  else if (status.data.state !== "ready") body = <SemanticRequired status={status.data} />;
  else if (results.isError) body = <ErrorMessage error={results.error} />;
  else if (!results.data) body = <Note>{t("search.meaning_searching")}</Note>;
  else if (results.data.length === 0) body = <Note>{t("search.no_results")}</Note>;
  else
    body = (
      <>
        <RelatedVerseList verses={results.data} />
        <Button asChild variant="outline" size="sm" className="justify-self-start">
          <Link to="/search" search={{ q, mode: "meaning" }}>
            {t("search.meaning_see_all")}
          </Link>
        </Button>
      </>
    );
  return (
    <section className="grid gap-3">
      <h2 className="text-sm font-medium">{t("search.group_meaning")}</h2>
      {body}
    </section>
  );
}

/** Verses closest in meaning, closest first; "Show more" asks for a longer list. */
function MeaningResults({ q }: { q: string }) {
  const { t } = useTranslation();
  const status = useSemanticStatus();
  const [limit, setLimit] = useState(MEANING_PAGE);
  const ready = status.data?.state === "ready";
  const results = useSearchMeaning(
    { query: q, translationId: useReaderTranslation(), limit },
    ready,
  );
  if (status.isError) return <ErrorMessage error={status.error} />;
  if (!status.data) return <Note>{t("search.searching")}</Note>;
  if (status.data.state !== "ready") return <SemanticRequired status={status.data} />;
  const provider = status.data.provider;
  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">
        {t("search.meaning_hint")}
        {provider && ` ${t("semantic.remote", { provider: PROVIDER_NAMES[provider] })}`}
      </p>
      {results.isError && <ErrorMessage error={results.error} />}
      {!results.isError && !results.data && <Note>{t("search.meaning_searching")}</Note>}
      {results.data?.length === 0 && <Note>{t("search.no_results")}</Note>}
      {results.data && results.data.length > 0 && (
        <>
          <RelatedVerseList verses={results.data} />
          {results.data.length >= limit && limit < MEANING_MAX && (
            <Button
              variant="outline"
              size="sm"
              className="justify-self-start"
              onClick={() => {
                setLimit(Math.min(limit + MEANING_PAGE, MEANING_MAX));
              }}
            >
              {t("search.show_more")}
            </Button>
          )}
        </>
      )}
    </div>
  );
}

/** The full list with filters; "Show more" asks for a longer list. */
function TextResults({ q, scope }: { q: string; scope: SearchScope }) {
  const { t } = useTranslation();
  const translations = useTranslations();
  const [limit, setLimit] = useState(PAGE);
  const [filters, setFilters] = useState(NO_FILTERS);
  const [packIds, setPackIds] = useState<string[]>([]);
  const results = useSearchText({ query: q, scope, packIds, filters, limit });

  return (
    <div className="grid gap-4">
      <Filters
        filters={filters}
        onChange={(next) => {
          setFilters(next);
          setLimit(PAGE);
        }}
      />
      {scope === "translations" && (translations.data?.length ?? 0) > 1 && (
        <PackFilter
          packs={translations.data ?? []}
          selected={packIds}
          onChange={(next) => {
            setPackIds(next);
            setLimit(PAGE);
          }}
        />
      )}
      {results.isError && <ErrorMessage error={results.error} />}
      {!results.isError && !results.data && <Note>{t("search.searching")}</Note>}
      {results.data?.total === 0 && <Note>{t("search.no_results")}</Note>}
      {results.data && results.data.total > 0 && (
        <section className="grid gap-3">
          <p role="status" className="text-sm text-muted-foreground">
            {t("search.total", { count: results.data.total })}
          </p>
          <HitList hits={results.data.hits} />
          {results.data.total > results.data.hits.length && (
            <Button
              variant="outline"
              size="sm"
              className="justify-self-start"
              onClick={() => {
                setLimit(limit + PAGE);
              }}
            >
              {t("search.show_more")}
            </Button>
          )}
        </section>
      )}
    </div>
  );
}

const REVELATIONS = ["any", "meccan", "medinan"] as const;

function parseSurahNumber(value: string): number | null {
  const parsed = Number(value);
  return value.trim() !== "" && Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

function Filters({
  filters,
  onChange,
}: {
  filters: SearchFilters;
  onChange: (filters: SearchFilters) => void;
}) {
  const { t } = useTranslation();
  const fromId = useId();
  const toId = useId();
  return (
    <div className="grid gap-3 rounded-lg border p-3 sm:grid-cols-2">
      <div className="grid gap-1.5">
        <span className="text-sm font-medium">{t("search.surah_range")}</span>
        <div className="flex items-center gap-2">
          <label htmlFor={fromId} className="text-xs text-muted-foreground">
            {t("search.surah_from")}
          </label>
          <Input
            id={fromId}
            type="number"
            min={1}
            max={114}
            className="w-20"
            value={filters.surah_from ?? ""}
            onChange={(event) => {
              onChange({ ...filters, surah_from: parseSurahNumber(event.target.value) });
            }}
          />
          <label htmlFor={toId} className="text-xs text-muted-foreground">
            {t("search.surah_to")}
          </label>
          <Input
            id={toId}
            type="number"
            min={1}
            max={114}
            className="w-20"
            value={filters.surah_to ?? ""}
            onChange={(event) => {
              onChange({ ...filters, surah_to: parseSurahNumber(event.target.value) });
            }}
          />
        </div>
      </div>
      <RadioField
        legend={t("search.revelation")}
        value={filters.revelation ?? "any"}
        options={REVELATIONS.map((value) => ({ value, label: t(`search.revelation_${value}`) }))}
        onChange={(value) => {
          const revelation: RevelationType | null = value === "any" ? null : value;
          onChange({ ...filters, revelation });
        }}
      />
    </div>
  );
}

/** Which translations to search; none checked means all of them. */
function PackFilter({
  packs,
  selected,
  onChange,
}: {
  packs: TranslationInfo[];
  selected: string[];
  onChange: (selected: string[]) => void;
}) {
  const { t, i18n } = useTranslation();
  const baseId = useId();
  return (
    <fieldset className="grid gap-1.5 rounded-lg border p-3">
      <legend className="px-1 text-sm font-medium">{t("search.packs")}</legend>
      <p className="text-xs text-muted-foreground">{t("search.packs_hint")}</p>
      {packs.map((pack) => (
        <div key={pack.id} className="flex items-center gap-2 text-sm">
          <Checkbox
            id={`${baseId}-${pack.id}`}
            checked={selected.includes(pack.id)}
            onCheckedChange={(checked) => {
              onChange(
                checked === true ? [...selected, pack.id] : selected.filter((id) => id !== pack.id),
              );
            }}
          />
          <label htmlFor={`${baseId}-${pack.id}`}>
            {translationTitle(pack, i18n.resolvedLanguage)}
          </label>
        </div>
      ))}
    </fieldset>
  );
}

function HitList({ hits }: { hits: SearchHit[] }) {
  return (
    <ul className="grid gap-2">
      {hits.map((hit) => (
        <li key={`${hit.pack_id ?? "ar"}-${String(hit.surah)}-${String(hit.ayah)}`}>
          <Hit hit={hit} />
        </li>
      ))}
    </ul>
  );
}

/** One result: opens the verse in the reader. */
function Hit({ hit }: { hit: SearchHit }) {
  const { i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const surahs = useSurahs();
  const translations = useTranslations();
  const surah = surahs.data?.find((item) => item.number === hit.surah);
  const pack = translations.data?.find((item) => item.id === hit.pack_id);
  const reference = surah ? verseReference(surahName(surah, language), hit) : verseNumber(hit);
  const isArabic = hit.pack_id === null;
  return (
    <div className="relative">
      <AddVerseButton verse={hit} className="absolute end-2 top-2" />
      <Link
        to="/read/$surah/$ayah"
        params={{ surah: hit.surah, ayah: hit.ayah }}
        className="grid gap-1 rounded-lg border p-3 transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
      >
        <span className="text-xs text-muted-foreground">
          {pack ? `${reference} · ${translationTitle(pack, language)}` : reference}
        </span>
        <span
          dir={isArabic ? "rtl" : (pack?.dir ?? "auto")}
          lang={isArabic ? "ar" : (pack?.lang ?? undefined)}
          className={isArabic ? "font-quran text-xl leading-loose" : "translation-text"}
        >
          {hit.parts.map((part, index) =>
            part.matched ? (
              // The parts of one result never change order.
              <mark key={index} className="rounded-sm bg-primary/15 text-foreground">
                {part.text}
              </mark>
            ) : (
              <span key={index}>{part.text}</span>
            ),
          )}
        </span>
      </Link>
    </div>
  );
}
