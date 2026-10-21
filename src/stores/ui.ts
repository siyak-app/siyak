import { create } from "zustand";

/** Interface language preference; `system` follows the operating system. */
export type LanguagePreference = "system" | "tr" | "en";

/** Colour theme preference; `system` follows the operating system. */
export type ThemePreference = "system" | "light" | "dark";

/** Tabs of the right-hand context panel (spec §13.2). */
export const CONTEXT_TABS = ["tafsir", "word", "related", "notes", "chat"] as const;

/** A tab of the context panel. */
export type ContextTab = (typeof CONTEXT_TABS)[number];

/** Size (in pixels) and visibility of a side pane. */
export interface PaneState {
  size: number;
  collapsed: boolean;
}

/**
 * UI preferences, keyed by their `user.db` setting key so that persisting a
 * change is a single `set_setting(key, value)` call.
 */
export interface UiPreferences {
  "ui.language": LanguagePreference;
  "ui.theme": ThemePreference;
  "layout.left_pane": PaneState;
  "layout.right_panel": PaneState & { tab: ContextTab };
}

/** A preference key. */
export type PreferenceKey = keyof UiPreferences;

/**
 * Defaults used until settings are loaded and for missing or invalid values.
 * TODO(M2): defaults come from the experience profile (spec §13.2).
 */
export const DEFAULT_PREFERENCES: UiPreferences = {
  "ui.language": "system",
  "ui.theme": "system",
  "layout.left_pane": { size: 260, collapsed: false },
  "layout.right_panel": { size: 340, collapsed: false, tab: "tafsir" },
};

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const isPane = (value: unknown): value is PaneState =>
  isObject(value) &&
  typeof value.size === "number" &&
  value.size > 0 &&
  typeof value.collapsed === "boolean";

const isOneOf =
  <T extends string>(options: readonly T[]) =>
  (value: unknown): value is T =>
    options.some((option) => option === value);

const VALIDATORS: { [K in PreferenceKey]: (value: unknown) => value is UiPreferences[K] } = {
  "ui.language": isOneOf(["system", "tr", "en"]),
  "ui.theme": isOneOf(["system", "light", "dark"]),
  "layout.left_pane": isPane,
  "layout.right_panel": (value): value is UiPreferences["layout.right_panel"] =>
    isPane(value) && isObject(value) && isOneOf(CONTEXT_TABS)(value.tab),
};

function parsePreference<K extends PreferenceKey>(
  key: K,
  json: string | undefined,
): UiPreferences[K] {
  if (json === undefined) return DEFAULT_PREFERENCES[key];
  try {
    const value: unknown = JSON.parse(json);
    return VALIDATORS[key](value) ? value : DEFAULT_PREFERENCES[key];
  } catch {
    return DEFAULT_PREFERENCES[key];
  }
}

/**
 * Builds preferences from stored settings (`key → JSON text`), falling back
 * to defaults for anything missing or malformed so a bad value never breaks
 * the interface.
 */
export function preferencesFromSettings(settings: Partial<Record<string, string>>): UiPreferences {
  return {
    "ui.language": parsePreference("ui.language", settings["ui.language"]),
    "ui.theme": parsePreference("ui.theme", settings["ui.theme"]),
    "layout.left_pane": parsePreference("layout.left_pane", settings["layout.left_pane"]),
    "layout.right_panel": parsePreference("layout.right_panel", settings["layout.right_panel"]),
  };
}

interface UiState {
  /** False until stored settings have been loaded into the store. */
  hydrated: boolean;
  preferences: UiPreferences;
  hydrate: (preferences: UiPreferences) => void;
  setPreference: <K extends PreferenceKey>(key: K, value: UiPreferences[K]) => void;
}

/**
 * UI preferences only (spec §2: Zustand is not used for content data).
 * Changes are persisted by `usePersistPreferences`.
 */
export const useUiStore = create<UiState>()((set) => ({
  hydrated: false,
  preferences: DEFAULT_PREFERENCES,
  hydrate: (preferences) => {
    set({ hydrated: true, preferences });
  },
  setPreference: (key, value) => {
    set((state) => ({ preferences: { ...state.preferences, [key]: value } }));
  },
}));
