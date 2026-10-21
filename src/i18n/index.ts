import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./en.json";
import tr from "./tr.json";

/** Languages the interface is translated into (spec §1). */
export const SUPPORTED_LANGUAGES = ["tr", "en"] as const;

/** A supported interface language. */
export type Language = (typeof SUPPORTED_LANGUAGES)[number];

/**
 * Picks the interface language from the operating system's preferred
 * languages: Turkish if the system prefers it, English otherwise.
 */
export function systemLanguage(preferred: readonly string[] = navigator.languages): Language {
  const match = preferred
    .map((tag) => tag.toLowerCase().split("-")[0])
    .find((code): code is Language => SUPPORTED_LANGUAGES.some((lang) => lang === code));
  return match ?? "en";
}

/** Translation resources, also used to type-check translation keys. */
export const resources = {
  en: { translation: en },
  tr: { translation: tr },
} as const;

void i18n.use(initReactI18next).init({
  resources,
  lng: systemLanguage(),
  fallbackLng: "en",
  supportedLngs: SUPPORTED_LANGUAGES,
  // React already escapes rendered values.
  interpolation: { escapeValue: false },
});

export default i18n;
