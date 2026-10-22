import { SURAH_COUNT } from "@/app/readParams";
import type { Surah } from "@/lib/bindings";
import { fold, foldName } from "@/lib/fold";

/** Where "Go to" leads: a surah, or a verse of it. */
export interface GoToTarget {
  surah: number;
  ayah?: number;
}

/** Why an input leads nowhere. */
export type GoToError = "surah_out_of_range" | "ayah_out_of_range" | "no_match";

export type GoToResult = { ok: true; target: GoToTarget } | { ok: false; error: GoToError };

/** `2`, `2:255`, `2 255`, `2.255`, `2/255`. */
const NUMERIC = /^(\d+)(?:\s*[:./\s]\s*(\d+))?$/u;
/** A name, optionally followed by a verse number: `Bakara 255`, `Al-Baqara:255`. */
const NAMED = /^(.*?\D)\s*(?:[:./\s]\s*(\d+))?$/u;

/** Arabic-Indic and Extended Arabic-Indic digits as ASCII digits. */
function asciiDigits(text: string): string {
  return text.replace(/[٠-٩۰-۹]/gu, (digit) => String((digit.codePointAt(0) ?? 0) & 0xf));
}

/** The surah called `name` (any of its names), or the only one starting so. */
function findSurah(name: string, surahs: readonly Surah[]): Surah | undefined {
  const folded = fold(name);
  const latin = foldName(name);
  if (folded === "") return undefined;
  const keys = (surah: Surah) => [
    fold(surah.name_ar),
    fold(surah.name_tr),
    foldName(surah.name_tr),
    foldName(surah.name_en),
  ];
  const exact = surahs.find((surah) => keys(surah).some((key) => key === folded || key === latin));
  if (exact) return exact;
  const prefixed = surahs.filter((surah) =>
    keys(surah).some((key) => key.startsWith(folded) || key.startsWith(latin)),
  );
  return prefixed.length === 1 ? prefixed[0] : undefined;
}

function target(surah: number, ayah: number | undefined, surahs: readonly Surah[]): GoToResult {
  if (surah < 1 || surah > SURAH_COUNT) return { ok: false, error: "surah_out_of_range" };
  if (ayah === undefined) return { ok: true, target: { surah } };
  const count = surahs.find((item) => item.number === surah)?.ayah_count;
  if (ayah < 1 || (count !== undefined && ayah > count)) {
    return { ok: false, error: "ayah_out_of_range" };
  }
  return { ok: true, target: { surah, ayah } };
}

/**
 * Parses the "Go to" input of the command palette: a surah number or name,
 * optionally with a verse number. `2:255`, `2 255`, `Bakara 255`,
 * `Baqarah 255`, `Al-Baqara:255` and `البقرة ٢٥٥` all lead to 2:255.
 * Returns `null` for empty input.
 */
export function parseGoTo(input: string, surahs: readonly Surah[]): GoToResult | null {
  const text = asciiDigits(input.trim());
  if (text === "") return null;

  const numeric = NUMERIC.exec(text);
  if (numeric) {
    const ayah = numeric[2] === undefined ? undefined : Number(numeric[2]);
    return target(Number(numeric[1]), ayah, surahs);
  }

  const named = NAMED.exec(text);
  const surah = named?.[1] === undefined ? undefined : findSurah(named[1], surahs);
  if (!named || !surah) return { ok: false, error: "no_match" };
  return target(surah.number, named[2] === undefined ? undefined : Number(named[2]), surahs);
}
