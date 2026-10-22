import { useTranslation } from "react-i18next";

import { useSetSettings, useTranslations } from "@/lib/queries";
import type { ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { profileDefaults } from "./profileDefaults";

/**
 * Applies an experience profile: writes all of its defaults in one
 * `set_settings` call, then updates the UI store. Used by the first-launch
 * question and by the profile resets in Settings.
 */
export function useApplyProfile() {
  const { i18n } = useTranslation();
  const translations = useTranslations();
  const save = useSetSettings();

  const apply = (profile: ExperienceProfile, onDone?: () => void) => {
    const { preferences, setPreferences } = useUiStore.getState();
    const values = profileDefaults(
      profile,
      translations.data ?? [],
      i18n.resolvedLanguage ?? "en",
      preferences["layout.right_panel"],
    );
    save.mutate(values, {
      onSuccess: () => {
        setPreferences(values);
        onDone?.();
      },
    });
  };

  return {
    apply,
    /** True while the installed translations load or a profile is being saved. */
    busy: translations.isPending || save.isPending,
    error: save.isError ? save.error : null,
  };
}
