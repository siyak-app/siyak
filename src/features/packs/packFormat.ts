import type { Catalog, JobView, LocalizedTitle, PackView } from "@/lib/bindings";
import type { PackCategory } from "@/stores/ui";

/** A byte count for people, e.g. "1.9 MB" / "1,9 MB" (decimal units, like file managers). */
export function formatBytes(bytes: number | null, language: string | undefined): string {
  const units = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;
  let value = bytes ?? 0;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return new Intl.NumberFormat(language, {
    style: "unit",
    unit: units[unit],
    unitDisplay: "short",
    maximumFractionDigits: value < 10 && unit > 0 ? 1 : 0,
  }).format(value);
}

/** A pack title in the interface language, falling back to the English title. */
export function packTitle(
  title: LocalizedTitle,
  fallback: string,
  language: string | undefined,
): string {
  return (language === "tr" ? title.tr : title.en) ?? fallback;
}

/** `catalog` with the job of pack `id` replaced (`null` clears it). */
export function withJob(catalog: Catalog, id: string, job: JobView | null): Catalog {
  return {
    ...catalog,
    packs: catalog.packs.map((pack) => (pack.id === id ? { ...pack, job } : pack)),
  };
}

/** How far a running job is, 0–1, or `null` if unknown. */
export function jobFraction(job: JobView): number | null {
  // Byte counts are `f64` in Rust, which specta types as `number | null`.
  const total = job.state === "running" ? (job.total ?? 0) : 0;
  if (job.state !== "running" || total <= 0) return null;
  return Math.min(1, (job.done ?? 0) / total);
}

/** Pack types with a section of their own on the Packs screen. */
const LISTED_KINDS: readonly string[] = ["translation", "tafsir", "morphology", "vectors", "audio"];

/**
 * The section of the Packs screen that lists `pack`. A model pack is part of
 * the vector pack that needs it, so it has no row (`null`) unless it is
 * installed and nothing needs it any more; then it is listed to be removed.
 */
export function packCategory(pack: PackView, catalog: Catalog): PackCategory | null {
  if (pack.kind === "model") {
    const needed = catalog.packs.some((other) => other.requires.includes(pack.id));
    return pack.installed && !needed ? "vectors" : null;
  }
  return LISTED_KINDS.includes(pack.kind) ? (pack.kind as PackCategory) : "other";
}

/** The packs installed with `pack` (a vector pack's model), as the catalog lists them. */
export function requiredPacks(pack: PackView, catalog: Catalog): PackView[] {
  return pack.requires.flatMap((id) => catalog.packs.filter((p) => p.id === id));
}

/** Download size of the packs installed with `pack` that are not installed yet. */
export function requiredSize(pack: PackView, catalog: Catalog): number {
  return requiredPacks(pack, catalog)
    .filter((p) => !p.installed)
    .reduce((sum, p) => sum + (p.size ?? 0), 0);
}

/** The packs installed with `pack` that no other installed pack needs. */
export function orphanedBy(pack: PackView, catalog: Catalog): PackView[] {
  return requiredPacks(pack, catalog).filter(
    (required) =>
      required.installed &&
      !required.bundled &&
      !catalog.packs.some(
        (other) => other.id !== pack.id && other.installed && other.requires.includes(required.id),
      ),
  );
}

/**
 * `text` for matching: lower case, without diacritics, transliteration
 * apostrophes or the dotless ı, so "elmalili" finds "Elmalılı" and "quran"
 * finds "Qurʾān".
 */
export function foldForSearch(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[\p{M}\u02BE\u02BF'’`]/gu, "")
    .replace(/ı/g, "i")
    .toLowerCase();
}

/** Whether `query` occurs in a title or the author of `pack`. */
export function matchesQuery(pack: PackView, query: string): boolean {
  const needle = foldForSearch(query.trim());
  if (needle === "") return true;
  return [pack.title.tr, pack.title.en, pack.title_fallback, pack.author].some(
    (field) => field != null && foldForSearch(field).includes(needle),
  );
}

/**
 * `packs` grouped by content language: the interface language first, then
 * the others by name, packs without a language last.
 */
export function groupByLanguage(
  packs: readonly PackView[],
  language: string | undefined,
): [string | null, PackView[]][] {
  const groups = new Map<string | null, PackView[]>();
  for (const pack of packs) groups.set(pack.lang, [...(groups.get(pack.lang) ?? []), pack]);
  const names = languageNames(language);
  const rank = (lang: string | null) => (lang === null ? 2 : lang === language ? 0 : 1);
  return [...groups].sort(
    ([a], [b]) => rank(a) - rank(b) || (names(a) ?? "").localeCompare(names(b) ?? "", language),
  );
}

/** A function naming a language code in `language`, e.g. "ar" → "Arabic". */
export function languageNames(
  language: string | undefined,
): (code: string | null) => string | null {
  const names = new Intl.DisplayNames(language ?? "en", { type: "language" });
  return (code) => {
    if (code === null) return null;
    try {
      return names.of(code) ?? code;
    } catch {
      return code;
    }
  };
}
