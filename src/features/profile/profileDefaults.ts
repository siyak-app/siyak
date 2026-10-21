import type { TranslationInfo } from "@/lib/bindings";
import type { ContextPanelState, ExperienceProfile, Settings } from "@/lib/settings";

/**
 * The settings an experience profile sets (spec §13.2). All of them are
 * written when the profile is chosen, including those whose features arrive
 * in later milestones, so later code only reads them.
 */
export const PROFILE_KEYS = [
  "experience_profile",
  "layout.right_panel",
  "reader.translations",
  "reader.actions",
  "word.click",
  "word.underline",
  "ui.technical_terms",
  "search.mode_chips",
  "reader.citation_action",
] as const satisfies readonly (keyof Settings)[];

/** A setting an experience profile sets. */
export type ProfileKey = (typeof PROFILE_KEYS)[number];

/** The values an experience profile sets. */
export type ProfileSettings = Pick<Settings, ProfileKey>;

/** How many translations each profile shows by default. */
const TRANSLATION_COUNT: Record<ExperienceProfile, number> = { reader: 1, researcher: 2 };

/**
 * Picks the default translations: those in the interface language first,
 * then bundled ones, then by id.
 */
export function defaultTranslations(
  installed: readonly TranslationInfo[],
  count: number,
  language: string,
): string[] {
  const inLanguage = (t: TranslationInfo) => (t.lang?.split("-")[0] === language ? 0 : 1);
  return [...installed]
    .sort(
      (a, b) =>
        inLanguage(a) - inLanguage(b) ||
        Number(b.bundled) - Number(a.bundled) ||
        a.id.localeCompare(b.id),
    )
    .slice(0, count)
    .map((t) => t.id);
}

/**
 * Every default of `profile`. `rightPanel` keeps the current pane size;
 * `language` is the resolved interface language (`tr` or `en`).
 */
export function profileDefaults(
  profile: ExperienceProfile,
  installed: readonly TranslationInfo[],
  language: string,
  rightPanel: ContextPanelState,
): ProfileSettings {
  const researcher = profile === "researcher";
  return {
    experience_profile: profile,
    "layout.right_panel": researcher
      ? { ...rightPanel, collapsed: false, tab: "word" }
      : { ...rightPanel, collapsed: true },
    "reader.translations": defaultTranslations(installed, TRANSLATION_COUNT[profile], language),
    "reader.actions": researcher ? "labeled" : "hover",
    "word.click": researcher ? "panel" : "card",
    "word.underline": researcher ? "dotted" : "none",
    "ui.technical_terms": researcher ? "inline" : "tooltip",
    "search.mode_chips": researcher,
    "reader.citation_action": researcher,
  };
}
