import { useTranslation } from "react-i18next";

import { useSetSettings, useTranslations } from "@/lib/queries";
import type { ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { profileDefaults } from "./profileDefaults";

/**
 * Applies an experience profile: writes its defaults in one `set_settings`
 * call, then updates the UI store. The first-launch question writes all of
 * them; switching mode from the header (`keepTranslations`) leaves the
 * translations the user has chosen alone.
 */
export function useApplyProfile() {
  const { i18n } = useTranslation();
  const translations = useTranslations();
  const save = useSetSettings();

  const apply = (
    profile: ExperienceProfile,
    onDone?: () => void,
    { keepTranslations = false }: { keepTranslations?: boolean } = {},
  ) => {
    const { preferences, setPreferences } = useUiStore.getState();
    const defaults = profileDefaults(
      profile,
      translations.data ?? [],
      i18n.resolvedLanguage ?? "en",
      preferences["layout.right_panel"],
    );
    const values = keepTranslations
      ? { ...defaults, "reader.translations": preferences["reader.translations"] }
      : defaults;
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
