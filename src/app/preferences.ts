import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { systemLanguage } from "@/i18n";
import { queryKeys, useSetSetting, useSettings } from "@/lib/queries";
import { serializeSetting } from "@/lib/settings";
import {
  preferencesFromSettings,
  useUiStore,
  type PreferenceKey,
  type UiPreferences,
} from "@/stores/ui";

/** Pane resizing fires continuously; wait for the user to pause before saving. */
const SAVE_DELAY_MS = 400;

/**
 * Loads stored preferences into the UI store once, then saves every change
 * back to `user.db` (spec §13.2: the layout is persisted in `settings`).
 * A value that is already stored (e.g. written by `set_settings`) is not
 * written again.
 *
 * If settings cannot be loaded the defaults are used, so the interface still
 * appears and the failing feature reports its own error.
 */
export function usePreferencesSync(): void {
  const settings = useSettings();
  const hydrated = useUiStore((state) => state.hydrated);
  const hydrate = useUiStore((state) => state.hydrate);
  const { mutate: saveSetting } = useSetSetting();
  const queryClient = useQueryClient();

  useEffect(() => {
    if (hydrated) return;
    if (settings.isSuccess) hydrate(preferencesFromSettings(settings.data));
    else if (settings.isError) hydrate(preferencesFromSettings({}));
  }, [hydrated, hydrate, settings.isSuccess, settings.isError, settings.data]);

  const timers = useRef(new Map<PreferenceKey, ReturnType<typeof setTimeout>>());
  useEffect(() => {
    const pending = timers.current;
    const unsubscribe = useUiStore.subscribe((state, previous) => {
      if (!previous.hydrated) return;
      for (const key of Object.keys(state.preferences) as PreferenceKey[]) {
        if (state.preferences[key] === previous.preferences[key]) continue;
        clearTimeout(pending.get(key));
        pending.set(
          key,
          setTimeout(() => {
            pending.delete(key);
            const value = useUiStore.getState().preferences[key];
            const stored = queryClient.getQueryData<Partial<Record<string, string>>>(
              queryKeys.settings,
            )?.[key];
            if (stored === serializeSetting(key, value)) return;
            saveSetting({ key, value });
          }, SAVE_DELAY_MS),
        );
      }
    });
    return () => {
      unsubscribe();
      for (const timer of pending.values()) clearTimeout(timer);
    };
  }, [saveSetting, queryClient]);
}

/** Keeps i18next and `<html lang>` in line with the language preference. */
export function useApplyLanguage(preference: UiPreferences["ui.language"]): void {
  const { i18n } = useTranslation();
  useEffect(() => {
    const language = preference === "system" ? systemLanguage() : preference;
    document.documentElement.lang = language;
    if (i18n.language !== language) void i18n.changeLanguage(language);
  }, [preference, i18n]);
}

/**
 * Sets the theme class on `<html>` (`dark` or `sepia`; none for light),
 * following the OS unless overridden.
 */
export function useApplyTheme(preference: UiPreferences["ui.theme"]): void {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = preference === "dark" || (preference === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
      document.documentElement.classList.toggle("sepia", preference === "sepia");
    };
    apply();
    media.addEventListener("change", apply);
    return () => {
      media.removeEventListener("change", apply);
    };
  }, [preference]);
}
