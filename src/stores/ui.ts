import { create } from "zustand";

import {
  DEFAULT_SETTINGS,
  parseSettings,
  type SettingKey,
  type Settings,
  type VersePosition,
} from "@/lib/settings";

/**
 * UI preferences: the settings of `@/lib/settings`, keyed by their `user.db`
 * setting key so that persisting a change is a single `set_setting` call.
 */
export type UiPreferences = Settings;

/** A preference key. */
export type PreferenceKey = SettingKey;

/** Defaults used until settings are loaded and for missing or invalid values. */
export const DEFAULT_PREFERENCES: UiPreferences = DEFAULT_SETTINGS;

/**
 * Builds preferences from stored settings (`key → JSON text`), falling back
 * to defaults for anything missing or malformed so a bad value never breaks
 * the interface.
 */
export const preferencesFromSettings = parseSettings;

interface UiState {
  /** False until stored settings have been loaded into the store. */
  hydrated: boolean;
  preferences: UiPreferences;
  /** The verse the context panel follows; not persisted. */
  selectedVerse: VersePosition | null;
  /** Whether the Packs dialog is open; other screens may open it. Not persisted. */
  packsDialogOpen: boolean;
  hydrate: (preferences: UiPreferences) => void;
  setPreference: <K extends PreferenceKey>(key: K, value: UiPreferences[K]) => void;
  setPreferences: (values: Partial<UiPreferences>) => void;
  selectVerse: (verse: VersePosition | null) => void;
  setPacksDialogOpen: (open: boolean) => void;
}

/**
 * UI preferences and selection only (spec §2: Zustand is not used for content
 * data). Preference changes are persisted by `usePreferencesSync`.
 */
export const useUiStore = create<UiState>()((set) => ({
  hydrated: false,
  preferences: DEFAULT_PREFERENCES,
  selectedVerse: null,
  packsDialogOpen: false,
  hydrate: (preferences) => {
    set({ hydrated: true, preferences });
  },
  setPreference: (key, value) => {
    set((state) => ({ preferences: { ...state.preferences, [key]: value } }));
  },
  setPreferences: (values) => {
    set((state) => ({ preferences: { ...state.preferences, ...values } }));
  },
  selectVerse: (verse) => {
    set({ selectedVerse: verse });
  },
  setPacksDialogOpen: (open) => {
    set({ packsDialogOpen: open });
  },
}));
