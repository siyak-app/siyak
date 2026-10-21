import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { systemLanguage } from "@/i18n";
import { useSetSetting, useSettings } from "@/lib/queries";
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
 *
 * If settings cannot be loaded the defaults are used, so the interface still
 * appears and the failing feature reports its own error.
 */
export function usePreferencesSync(): void {
  const settings = useSettings();
  const hydrated = useUiStore((state) => state.hydrated);
  const hydrate = useUiStore((state) => state.hydrate);
  const { mutate: saveSetting } = useSetSetting();

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
            saveSetting({ key, value: useUiStore.getState().preferences[key] });
          }, SAVE_DELAY_MS),
        );
      }
    });
    return () => {
      unsubscribe();
      for (const timer of pending.values()) clearTimeout(timer);
    };
  }, [saveSetting]);
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

/** Toggles the `dark` class on `<html>`, following the OS unless overridden. */
export function useApplyTheme(preference: UiPreferences["ui.theme"]): void {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = preference === "dark" || (preference === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
    };
    apply();
    media.addEventListener("change", apply);
    return () => {
      media.removeEventListener("change", apply);
    };
  }, [preference]);
}
