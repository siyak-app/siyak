import { Link, useNavigate } from "@tanstack/react-router";
import { BookOpenIcon } from "lucide-react";
import { type SyntheticEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { AddVerseButton } from "@/features/collections/AddToCollectionButton";
import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { packTitle } from "@/features/packs/packFormat";
import { surahName } from "@/features/reader/surahName";
import { TafsirParagraphs } from "@/features/tafsir/TafsirParagraphs";
import { verseIdOf, verseRange } from "@/features/tafsir/tafsirRefs";
import type {
  CompareTafsir,
  Surah,
  TafsirInfo,
  TafsirPassage,
  TranslationInfo,
  VerseWithTranslations,
} from "@/lib/bindings";
import { useCompare, useSurahs, useTafsirs, useTranslations } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { ColumnPicker } from "./ColumnPicker";
import { type CompareColumn, compareColumns, tafsirCells } from "./compareLayout";

/**
 * The compare screen (spec §13.3 item 2): translations and tafsirs of a verse
 * or range in parallel columns, with rows aligned per verse. Column choices
 * are kept in `compare.columns`.
 */
export function CompareView({ surah, from, to }: { surah: number; from: number; to: number }) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  const translations = useTranslations();
  const tafsirs = useTafsirs();
  const failed = surahs.error ?? translations.error ?? tafsirs.error;
  if (failed) return <ErrorMessage error={failed} />;
  if (!surahs.data || !translations.data || !tafsirs.data) {
    return <p className="p-6 text-sm text-muted-foreground">{t("compare.loading")}</p>;
  }
  const current = surahs.data.find((item) => item.number === surah);
  if (!current || to > current.ayah_count) {
    return (
      <p className="p-6 text-sm text-muted-foreground">
        {t("command_palette.errors.ayah_out_of_range")}
      </p>
    );
  }
  return (
    <CompareScreen
      surahs={surahs.data}
      surah={current}
      from={from}
      to={to}
      translations={translations.data}
      tafsirs={tafsirs.data}
    />
  );
}

function CompareScreen({
  surahs,
  surah,
  from,
  to,
  translations,
  tafsirs,
}: {
  surahs: readonly Surah[];
  surah: Surah;
  from: number;
  to: number;
  translations: readonly TranslationInfo[];
  tafsirs: readonly TafsirInfo[];
}) {
  const { t, i18n } = useTranslation();
  const stored = useUiStore((state) => state.preferences["compare.columns"]);
  const readerTranslations = useUiStore((state) => state.preferences["reader.translations"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const columns = compareColumns(stored, readerTranslations, translations, tafsirs);
  const compare = useCompare({
    from: verseIdOf(surahs, { surah: surah.number, ayah: from }),
    to: verseIdOf(surahs, { surah: surah.number, ayah: to }),
    translationIds: columns.flatMap((c) => (c.kind === "translation" ? [c.pack.id] : [])),
    tafsirIds: columns.flatMap((c) => (c.kind === "tafsir" ? [c.pack.id] : [])),
  });

  const number = String(surah.number);
  const range =
    from === to ? `${number}:${String(from)}` : `${number}:${String(from)}–${String(to)}`;
  const requested = to - from + 1;
  const shown = compare.data?.verses.length ?? requested;
  const attributions = columns.flatMap((c) =>
    c.kind === "tafsir" && c.pack.attribution ? [c.pack.attribution] : [],
  );

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="text-lg font-semibold">
          {surahName(surah, i18n.resolvedLanguage)} {range}
        </h1>
        <RangeForm
          key={`${number}-${String(from)}-${String(to)}`}
          surah={surah}
          from={from}
          to={to}
        />
        <span className="flex-1" />
        <AddVerseButton
          verse={{ surah: surah.number, ayah: from }}
          through={to}
          label={t("collections.add")}
        />
        <Button asChild variant="ghost" size="sm">
          <Link to="/read/$surah/$ayah" params={{ surah: surah.number, ayah: from }}>
            <BookOpenIcon aria-hidden />
            {t("compare.open_in_reader")}
          </Link>
        </Button>
        <ColumnPicker
          columns={columns}
          translations={translations}
          tafsirs={tafsirs}
          onChange={(ids) => {
            setPreference("compare.columns", ids);
          }}
        />
      </header>
      {shown < requested && (
        <p role="status" className="text-sm text-muted-foreground">
          {t("compare.truncated", { count: shown })}
        </p>
      )}
      {columns.length === 0 && (
        <p className="text-sm text-muted-foreground">{t("compare.no_columns")}</p>
      )}
      {compare.isError && <ErrorMessage error={compare.error} />}
      {compare.data && (
        <div className="min-h-0 flex-1 overflow-auto">
          <CompareTable
            columns={columns}
            verses={compare.data.verses}
            tafsirs={compare.data.tafsirs}
          />
        </div>
      )}
      {attributions.length > 0 && (
        <p className="text-xs text-muted-foreground">{attributions.join(" · ")}</p>
      )}
    </div>
  );
}

/** Chooses another range of the same surah. */
function RangeForm({ surah, from, to }: { surah: Surah; from: number; to: number }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const fromId = useId();
  const toId = useId();
  const [start, setStart] = useState(String(from));
  const [end, setEnd] = useState(String(to));
  const first = Number(start);
  const last = Number(end);
  const valid =
    Number.isInteger(first) &&
    Number.isInteger(last) &&
    first >= 1 &&
    first <= last &&
    last <= surah.ayah_count;

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    if (!valid) return;
    void navigate({
      to: "/compare/$surah/$range",
      params: { surah: surah.number, range: { from: first, to: last } },
    });
  };

  return (
    <form onSubmit={submit} className="flex items-center gap-1.5 text-xs text-muted-foreground">
      <label htmlFor={fromId}>{t("compare.from")}</label>
      <Input
        id={fromId}
        type="number"
        min={1}
        max={surah.ayah_count}
        value={start}
        className="h-7 w-16"
        onChange={(event) => {
          setStart(event.target.value);
        }}
      />
      <label htmlFor={toId}>{t("compare.to")}</label>
      <Input
        id={toId}
        type="number"
        min={1}
        max={surah.ayah_count}
        value={end}
        className="h-7 w-16"
        onChange={(event) => {
          setEnd(event.target.value);
        }}
      />
      <Button type="submit" variant="outline" size="sm" disabled={!valid}>
        {t("compare.show")}
      </Button>
    </form>
  );
}

function CompareTable({
  columns,
  verses,
  tafsirs,
}: {
  columns: readonly CompareColumn[];
  verses: readonly VerseWithTranslations[];
  tafsirs: readonly CompareTafsir[];
}) {
  const { t, i18n } = useTranslation();
  const cellsByPack = new Map(
    tafsirs.map((tafsir) => [tafsir.pack_id, tafsirCells(tafsir.passages, verses)]),
  );
  return (
    <table className="w-full border-collapse text-start text-sm">
      <thead>
        <tr>
          <th
            scope="col"
            className="sticky top-0 z-10 min-w-64 border-b bg-background p-2 text-start font-medium"
          >
            {t("compare.verse_column")}
          </th>
          {columns.map(({ pack }) => (
            <th
              key={pack.id}
              scope="col"
              className="sticky top-0 z-10 min-w-72 border-b bg-background p-2 text-start font-medium"
            >
              {packTitle(pack.titles, pack.title, i18n.resolvedLanguage)}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {verses.map((verse, row) => (
          <tr key={verse.id} className="border-b align-top">
            <th scope="row" className="p-2 text-start font-normal">
              <Link
                to="/read/$surah/$ayah"
                params={{ surah: verse.surah, ayah: verse.ayah }}
                className="text-xs text-muted-foreground hover:underline"
              >
                {`${String(verse.surah)}:${String(verse.ayah)}`}
              </Link>
              <p dir="rtl" lang="ar" className="arabic-text">
                {verse.text_uthmani}
              </p>
            </th>
            {columns.map((column) =>
              column.kind === "translation" ? (
                <TranslationCell key={column.pack.id} pack={column.pack} verse={verse} />
              ) : (
                <TafsirTableCell
                  key={column.pack.id}
                  pack={column.pack}
                  cell={cellsByPack.get(column.pack.id)?.[row]}
                />
              ),
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function TranslationCell({ pack, verse }: { pack: TranslationInfo; verse: VerseWithTranslations }) {
  const text = verse.translations.find((item) => item.pack_id === pack.id)?.text;
  return (
    <td dir={pack.dir} lang={pack.lang ?? undefined} className="translation-text p-2 text-start">
      {text}
    </td>
  );
}

function TafsirTableCell({
  pack,
  cell,
}: {
  pack: TafsirInfo;
  /** `null` where a passage from an earlier row continues. */
  cell: ReturnType<typeof tafsirCells>[number] | undefined;
}) {
  const { t } = useTranslation();
  if (cell === null) return null;
  if (!cell?.passage) {
    return (
      <td rowSpan={cell?.span} className="p-2 text-xs text-muted-foreground">
        {t("tafsir.no_commentary")}
      </td>
    );
  }
  return (
    <td rowSpan={cell.span} className="border-s p-2">
      <PassageCell pack={pack} passage={cell.passage} />
    </td>
  );
}

/** A passage in a scrolling cell: long commentary must not stretch the rows. */
function PassageCell({ pack, passage }: { pack: TafsirInfo; passage: TafsirPassage }) {
  const { t, i18n } = useTranslation();
  const { verse_from: start, verse_to: end } = passage;
  const isRange = start.surah !== end.surah || start.ayah !== end.ayah;
  return (
    <div
      tabIndex={0}
      aria-label={packTitle(pack.titles, pack.title, i18n.resolvedLanguage)}
      className="max-h-96 overflow-y-auto rounded-md focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
    >
      {isRange && (
        <p className="pb-2 text-xs text-muted-foreground">
          {t("tafsir.covers", { range: verseRange(start, end) })}
        </p>
      )}
      <TafsirParagraphs tafsir={pack} passage={passage} />
    </div>
  );
}
