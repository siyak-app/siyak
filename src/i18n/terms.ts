import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import type { Settings } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/**
 * Terms with a plain and a technical label (`term.<name>.plain` /
 * `term.<name>.technical`, spec §13.2, `docs/glossary.md`).
 */
export const TERMS = [
  "lemma",
  "root",
  "concordance",
  "semantic_search",
  "segment",
  "morphology",
  "part_of_speech",
] as const;

/** A term with a plain and a technical label. */
export type TermName = (typeof TERMS)[number];

/** How technical terms are shown (`ui.technical_terms`). */
export type TermMode = Settings["ui.technical_terms"];

/**
 * The label of `name`: the plain wording, or "plain (technical)" when
 * technical terms are shown inline. A term whose two forms are the same is
 * shown once.
 */
export function termLabel(t: TFunction, name: TermName, mode: TermMode): string {
  const plain = t(`term.${name}.plain`);
  const technical = t(`term.${name}.technical`);
  if (mode === "tooltip" || plain === technical) return plain;
  return t("term_format.inline", { plain, technical });
}

/** The label of a term according to the `ui.technical_terms` setting. */
export function useTerm(name: TermName): string {
  const { t } = useTranslation();
  const mode = useUiStore((state) => state.preferences["ui.technical_terms"]);
  return termLabel(t, name, mode);
}
