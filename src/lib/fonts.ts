/**
 * The bundled typefaces the reader can use: their CSS font stacks and names.
 * The faces themselves are imported in `main.tsx` (SIL OFL, packaged by
 * Fontsource); the CSP allows no remote fonts (spec §13.4, §14.3).
 *
 * @packageDocumentation
 */

import type { ArabicFont, TranslationFont } from "./settings";

/** A typeface: its CSS font stack and its name, the same in every language. */
export interface FontFace {
  family: string;
  name: string;
}

/** Typefaces of the Arabic text. */
export const ARABIC_FONT_FACES: Record<ArabicFont, FontFace> = {
  "amiri-quran": { family: '"Amiri Quran", serif', name: "Amiri Quran" },
  amiri: { family: '"Amiri", serif', name: "Amiri" },
  "scheherazade-new": { family: '"Scheherazade New", serif', name: "Scheherazade New" },
  "noto-naskh-arabic": { family: '"Noto Naskh Arabic", serif', name: "Noto Naskh Arabic" },
  "noto-kufi-arabic": { family: '"Noto Kufi Arabic", sans-serif', name: "Noto Kufi Arabic" },
};

/** Typefaces of translations; `system` follows the interface font. */
export const TRANSLATION_FONT_FACES: Record<TranslationFont, FontFace> = {
  system: { family: "var(--font-sans)", name: "" },
  inter: { family: '"Inter", sans-serif', name: "Inter" },
  literata: { family: '"Literata", serif', name: "Literata" },
  "noto-serif": { family: '"Noto Serif", serif', name: "Noto Serif" },
  lora: { family: '"Lora", serif', name: "Lora" },
  "source-serif": { family: '"Source Serif 4", serif', name: "Source Serif 4" },
};
