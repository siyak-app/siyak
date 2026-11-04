import type { TFunction } from "i18next";

import en from "@/i18n/en.json";
import type {
  ConcordanceTarget,
  LemmaInfo,
  RootInfo,
  RootSearch,
  WordDetail,
  WordFeature,
  WordSegment,
} from "@/lib/bindings";

/**
 * Feature keys shown to the reader, in display order. `prefix` and `suffix`
 * only repeat the segment's own tag (`siyak-data/docs/pack-format.md` §7).
 */
const SHOWN_FEATURES = [
  "person",
  "gender",
  "number",
  "aspect",
  "mood",
  "voice",
  "verb_form",
  "derivation",
  "state",
  "case",
  "special",
] as const;

type ShownFeature = (typeof SHOWN_FEATURES)[number];

const isShown = (key: string): key is ShownFeature => SHOWN_FEATURES.some((shown) => shown === key);

const isPosTag = (tag: string): tag is keyof typeof en.word.pos => tag in en.word.pos;

const isValueKey = (key: string): key is keyof typeof en.word.value => key in en.word.value;

/** The readable name of a part-of-speech tag; a tag without one stays as the source wrote it. */
export function posLabel(t: TFunction, tag: string): string {
  return isPosTag(tag) ? t(`word.pos.${tag}`) : tag;
}

/** A feature's readable label and value, in display order. */
export function featureRows(
  t: TFunction,
  features: readonly WordFeature[],
): { key: string; label: string; value: string }[] {
  const rows = [];
  for (const shown of SHOWN_FEATURES) {
    const feature = features.find((item) => item.key === shown);
    if (!feature || !isShown(feature.key)) continue;
    const valueKey = `${feature.key}_${feature.value}`;
    rows.push({
      key: feature.key,
      label: t(`word.feature.${feature.key}`),
      // Verb forms are Roman numerals in every language.
      value: isValueKey(valueKey) ? t(`word.value.${valueKey}`) : feature.value,
    });
  }
  return rows;
}

/** The segment a short summary of the word is about: the first stem with a root, else the first stem. */
export function mainSegment(word: WordDetail): WordSegment | undefined {
  const stems = word.segments.filter((segment) => segment.kind === "stem");
  return stems.find((segment) => segment.root) ?? stems[0] ?? word.segments[0];
}

/** The word's different roots, in order. */
export function wordRoots(word: WordDetail): RootInfo[] {
  const roots = new Map<number, RootInfo>();
  for (const { root } of word.segments) if (root) roots.set(root.id, root);
  return [...roots.values()];
}

/** The word's different lemmas, in order. */
export function wordLemmas(word: WordDetail): LemmaInfo[] {
  const lemmas = new Map<number, LemmaInfo>();
  for (const { lemma } of word.segments) if (lemma) lemmas.set(lemma.id, lemma);
  return [...lemmas.values()];
}

/**
 * Splits QAC's homograph index off a lemma (`عَاد2`): two different lemmas may
 * share a spelling, so the digit is shown, raised, rather than dropped.
 */
export function splitLemma(lemma: string): { text: string; index: string | null } {
  const match = /^(.*?)(\d+)$/su.exec(lemma);
  return match?.[1] && match[2]
    ? { text: match[1], index: match[2] }
    : { text: lemma, index: null };
}

/** The one result a root search can go straight to: a single root, else a single lemma. */
export function singleTarget({ roots, lemmas }: RootSearch): ConcordanceTarget | null {
  const [root] = roots;
  if (root && roots.length === 1) return { kind: "root", id: root.id };
  const [lemma] = lemmas;
  if (roots.length === 0 && lemma && lemmas.length === 1) return { kind: "lemma", id: lemma.id };
  return null;
}
