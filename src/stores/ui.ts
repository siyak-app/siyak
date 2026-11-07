import { create } from "zustand";

import {
  DEFAULT_SETTINGS,
  parseSettings,
  type SettingKey,
  type Settings,
  type VersePosition,
} from "@/lib/settings";
import type { ItemRef } from "@/lib/bindings";

/** Tabs of the Settings dialog, in display order. */
export const SETTINGS_TABS = [
  "appearance",
  "display",
  "language",
  "providers",
  "ai",
  "data",
  "privacy",
] as const;

/** A tab of the Settings dialog. */
export type SettingsTab = (typeof SETTINGS_TABS)[number];

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
  /** Id of the word the Word tab analyzes, in the selected verse; not persisted. */
  selectedWord: number | null;
  /** Whether the Packs dialog is open; other screens may open it. Not persisted. */
  packsDialogOpen: boolean;
  /** The open tab of the Settings dialog, `null` if it is closed. Not persisted. */
  settingsTab: SettingsTab | null;
  /** What the "Add to collection" dialog is adding, if it is open. Not persisted. */
  collectionPicker: ItemRef | null;
  /** The next Notes tab to show should focus its editor (the `n` key). Not persisted. */
  noteFocusPending: boolean;
  /** The chat session of the context panel's Chat tab; `null` for a new one. Not persisted. */
  chatSession: number | null;
  hydrate: (preferences: UiPreferences) => void;
  setPreference: <K extends PreferenceKey>(key: K, value: UiPreferences[K]) => void;
  setPreferences: (values: Partial<UiPreferences>) => void;
  selectVerse: (verse: VersePosition | null) => void;
  selectWord: (verse: VersePosition, wordId: number) => void;
  setPacksDialogOpen: (open: boolean) => void;
  setSettingsTab: (tab: SettingsTab | null) => void;
  openCollectionPicker: (item: ItemRef | null) => void;
  setNoteFocusPending: (pending: boolean) => void;
  setChatSession: (id: number | null) => void;
}

/**
 * UI preferences and selection only (spec §2: Zustand is not used for content
 * data). Preference changes are persisted by `usePreferencesSync`.
 */
export const useUiStore = create<UiState>()((set) => ({
  hydrated: false,
  preferences: DEFAULT_PREFERENCES,
  selectedVerse: null,
  selectedWord: null,
  packsDialogOpen: false,
  settingsTab: null,
  collectionPicker: null,
  noteFocusPending: false,
  chatSession: null,
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
    set((state) => {
      const same =
        state.selectedVerse?.surah === verse?.surah && state.selectedVerse?.ayah === verse?.ayah;
      return { selectedVerse: verse, selectedWord: same ? state.selectedWord : null };
    });
  },
  selectWord: (verse, wordId) => {
    set({ selectedVerse: verse, selectedWord: wordId });
  },
  setPacksDialogOpen: (open) => {
    set({ packsDialogOpen: open });
  },
  setSettingsTab: (tab) => {
    set({ settingsTab: tab });
  },
  openCollectionPicker: (item) => {
    set({ collectionPicker: item });
  },
  setNoteFocusPending: (pending) => {
    set({ noteFocusPending: pending });
  },
  setChatSession: (id) => {
    set({ chatSession: id });
  },
}));
