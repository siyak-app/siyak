import type { TranslationInfo } from "@/lib/bindings";

/** A translation's title in the interface language, or its published title. */
export function translationTitle(info: TranslationInfo, language: string | undefined): string {
  const localized = language === "tr" ? info.titles.tr : info.titles.en;
  return localized ?? info.title;
}
