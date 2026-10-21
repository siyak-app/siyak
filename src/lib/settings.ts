/**
 * The typed settings model: every `user.db` setting the interface reads, with
 * its validator and default.
 *
 * Settings cross the IPC boundary as `key → JSON text` (`get_settings`,
 * `set_setting`, `set_settings`). This module is the one place that parses and
 * serializes them; a missing or malformed value falls back to its default, so
 * a bad value never breaks the interface.
 *
 * @packageDocumentation
 */

/** Interface language preference; `system` follows the operating system. */
export type LanguagePreference = "system" | "tr" | "en";

/** Colour theme preference; `system` follows the operating system. */
export type ThemePreference = "system" | "light" | "dark" | "sepia";

/** The first-launch answer that sets the defaults below (spec §13.2). */
export type ExperienceProfile = "reader" | "researcher";

/** Tabs of the right-hand context panel (spec §13.2). */
export const CONTEXT_TABS = ["tafsir", "word", "related", "notes", "chat"] as const;

/** A tab of the context panel. */
export type ContextTab = (typeof CONTEXT_TABS)[number];

/** Size (in pixels) and visibility of a side pane. */
export interface PaneState {
  size: number;
  collapsed: boolean;
}

/** The right-hand context panel: pane state plus its open tab. */
export type ContextPanelState = PaneState & { tab: ContextTab };

/** Font size (CSS pixels) and line height (unitless) of a text block. */
export interface Typography {
  size: number;
  line_height: number;
}

/** A verse position, e.g. the last one read. */
export interface VersePosition {
  surah: number;
  ayah: number;
}

/** Allowed ranges of the typography settings. */
export const TYPOGRAPHY_LIMITS = {
  arabic: { size: [20, 48], line_height: [2, 3] },
  translation: { size: [12, 26], line_height: [1.3, 2.2] },
} as const;

/** Every setting, keyed by its `user.db` key. */
export interface Settings {
  "ui.language": LanguagePreference;
  "ui.theme": ThemePreference;
  "layout.left_pane": PaneState;
  "layout.right_panel": ContextPanelState;
  /** `null` until the first-launch question has been answered. */
  experience_profile: ExperienceProfile | null;
  /** Ids of the translation packs shown in the reader, in display order. */
  "reader.translations": string[];
  /** Verse actions as icons on hover, or always visible with labels. */
  "reader.actions": "hover" | "labeled";
  /** Word click: a small card, or the full analysis in the right panel (M5). */
  "word.click": "card" | "panel";
  /** Underline of clickable words (M5). */
  "word.underline": "none" | "dotted";
  /** Technical terms: in tooltips only, or inline in parentheses. */
  "ui.technical_terms": "tooltip" | "inline";
  /** Search mode chips (M4). */
  "search.mode_chips": boolean;
  /** "Copy citation" verse action (M6). */
  "reader.citation_action": boolean;
  "typography.arabic": Typography;
  "typography.translation": Typography;
  /** Where the reader was last, opened on the next launch. */
  "reader.last_position": VersePosition | null;
}

/** A setting key. */
export type SettingKey = keyof Settings;

interface SettingSpec<T> {
  default: T;
  validate: (value: unknown) => value is T;
}

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const isOneOf =
  <T extends string>(options: readonly T[]) =>
  (value: unknown): value is T =>
    options.some((option) => option === value);

const isBoolean = (value: unknown): value is boolean => typeof value === "boolean";

const isPositiveInteger = (value: unknown): value is number =>
  Number.isInteger(value) && (value as number) > 0;

const isPane = (value: unknown): value is PaneState =>
  isObject(value) &&
  typeof value.size === "number" &&
  value.size > 0 &&
  typeof value.collapsed === "boolean";

const isTypography =
  (limits: { size: readonly [number, number]; line_height: readonly [number, number] }) =>
  (value: unknown): value is Typography =>
    isObject(value) &&
    typeof value.size === "number" &&
    typeof value.line_height === "number" &&
    value.size >= limits.size[0] &&
    value.size <= limits.size[1] &&
    value.line_height >= limits.line_height[0] &&
    value.line_height <= limits.line_height[1];

const nullable =
  <T>(validate: (value: unknown) => value is T) =>
  (value: unknown): value is T | null =>
    value === null || validate(value);

/** Validator and default of every setting. */
export const SETTINGS: { [K in SettingKey]: SettingSpec<Settings[K]> } = {
  "ui.language": { default: "system", validate: isOneOf(["system", "tr", "en"]) },
  "ui.theme": { default: "system", validate: isOneOf(["system", "light", "dark", "sepia"]) },
  "layout.left_pane": { default: { size: 260, collapsed: false }, validate: isPane },
  "layout.right_panel": {
    default: { size: 340, collapsed: false, tab: "tafsir" },
    validate: (value): value is ContextPanelState =>
      isPane(value) && isObject(value) && isOneOf(CONTEXT_TABS)(value.tab),
  },
  experience_profile: { default: null, validate: nullable(isOneOf(["reader", "researcher"])) },
  "reader.translations": {
    default: ["en.pickthall"],
    validate: (value): value is string[] =>
      Array.isArray(value) && value.every((id) => typeof id === "string"),
  },
  "reader.actions": { default: "hover", validate: isOneOf(["hover", "labeled"]) },
  "word.click": { default: "card", validate: isOneOf(["card", "panel"]) },
  "word.underline": { default: "none", validate: isOneOf(["none", "dotted"]) },
  "ui.technical_terms": { default: "tooltip", validate: isOneOf(["tooltip", "inline"]) },
  "search.mode_chips": { default: false, validate: isBoolean },
  "reader.citation_action": { default: false, validate: isBoolean },
  "typography.arabic": {
    default: { size: 28, line_height: 2.2 },
    validate: isTypography(TYPOGRAPHY_LIMITS.arabic),
  },
  "typography.translation": {
    default: { size: 16, line_height: 1.7 },
    validate: isTypography(TYPOGRAPHY_LIMITS.translation),
  },
  "reader.last_position": {
    default: null,
    validate: nullable(
      (value): value is VersePosition =>
        isObject(value) && isPositiveInteger(value.surah) && isPositiveInteger(value.ayah),
    ),
  },
};

/** Every setting key, in declaration order. */
export const SETTING_KEYS = Object.keys(SETTINGS) as SettingKey[];

/** Defaults of every setting. */
export const DEFAULT_SETTINGS = Object.fromEntries(
  SETTING_KEYS.map((key) => [key, SETTINGS[key].default]),
) as unknown as Settings;

/** Parses one stored value, falling back to the default if it is missing or invalid. */
export function parseSetting<K extends SettingKey>(key: K, json: string | undefined): Settings[K] {
  const spec: SettingSpec<Settings[K]> = SETTINGS[key];
  if (json === undefined) return spec.default;
  try {
    const value: unknown = JSON.parse(json);
    return spec.validate(value) ? value : spec.default;
  } catch {
    return spec.default;
  }
}

/** Parses every setting from stored `key → JSON text` pairs. */
export function parseSettings(stored: Partial<Record<string, string>>): Settings {
  return Object.fromEntries(
    SETTING_KEYS.map((key) => [key, parseSetting(key, stored[key])]),
  ) as unknown as Settings;
}

/** Serializes a setting value to the JSON text stored in `user.db`. */
export function serializeSetting<K extends SettingKey>(_key: K, value: Settings[K]): string {
  return JSON.stringify(value);
}
