import type { Catalog, JobView, LocalizedTitle } from "@/lib/bindings";

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

/** Order of the type groups on the Packs screen; other types come last. */
export const KIND_ORDER = [
  "translation",
  "tafsir",
  "audio",
  "morphology",
  "vectors",
  "model",
] as const;
