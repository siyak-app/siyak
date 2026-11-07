import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { ARABIC_FONT_FACES, TRANSLATION_FONT_FACES } from "@/lib/fonts";
import {
  ARABIC_FONTS,
  DEFAULT_SETTINGS,
  type Theme,
  type ThemePreference,
  THEMES,
  TRANSLATION_FONTS,
  type Typography,
  TYPOGRAPHY_LIMITS,
} from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { type CardOption, RangeField, SelectCard, SettingsSection } from "./fields";

/** Quranic text for the previews (the basmala); content, not an interface string. */
const ARABIC_SAMPLE = "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ";

/** A short Arabic word that shows a typeface in the font list. */
const ARABIC_GLYPHS = "بِسْمِ";

/** The letters a theme swatch shows its text colour with; a sample, not an interface string. */
const SWATCH_TEXT = "Aa";

/** Line heights move in tenths. */
const LINE_HEIGHT_STEP = 0.1;

/** The built-in themes, named in the interface language. */
type BuiltInTheme = "light" | "dark" | "sepia";

/** Names of the palettes after editor themes: proper names, the same in every language. */
const THEME_NAMES: Record<Exclude<Theme, BuiltInTheme>, string> = {
  "github-light": "GitHub Light",
  "solarized-light": "Solarized Light",
  "gruvbox-light": "Gruvbox Light",
  "catppuccin-latte": "Catppuccin Latte",
  "github-dark": "GitHub Dark",
  "one-dark": "One Dark",
  "solarized-dark": "Solarized Dark",
  nord: "Nord",
  dracula: "Dracula",
  "gruvbox-dark": "Gruvbox Dark",
  "catppuccin-mocha": "Catppuccin Mocha",
  "tokyo-night": "Tokyo Night",
  "rose-pine": "Rosé Pine",
};

/** A theme in miniature: its background, text and accent, drawn with its own tokens. */
function ThemeSwatch({ theme }: { theme: Theme }) {
  return (
    <span
      aria-hidden
      data-theme={theme}
      className="flex h-7 w-12 shrink-0 items-center justify-center gap-1 rounded-md border bg-background text-xs font-semibold text-foreground"
    >
      {SWATCH_TEXT}
      <span className="size-2 rounded-full bg-primary" />
    </span>
  );
}

/** "Same as the system": half light, half dark. */
function SystemSwatch() {
  return (
    <span aria-hidden className="flex h-7 w-12 shrink-0 overflow-hidden rounded-md border">
      <span data-theme="light" className="flex-1 bg-background" />
      <span data-theme="dark" className="flex-1 bg-background" />
    </span>
  );
}

/** Theme, typefaces and sizes of the reader (spec §13.4). */
export function AppearanceSettings() {
  const { t, i18n } = useTranslation();
  const preferences = useUiStore((state) => state.preferences);
  const setPreference = useUiStore((state) => state.setPreference);
  const arabic = preferences["typography.arabic"];
  const translation = preferences["typography.translation"];

  const themeName = (theme: Theme) =>
    theme === "light" || theme === "dark" || theme === "sepia"
      ? t(`settings.theme_${theme}`)
      : THEME_NAMES[theme];
  const themes: CardOption<ThemePreference>[] = [
    { value: "system", label: t("settings.follow_system"), preview: <SystemSwatch /> },
    ...THEMES.map((theme) => ({
      value: theme,
      label: themeName(theme),
      preview: <ThemeSwatch theme={theme} />,
    })),
  ];

  const pixels = (value: number) => t("settings.pixels", { value });
  const lineHeight = (value: number) =>
    value.toLocaleString(i18n.resolvedLanguage, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    });

  const sizes = (
    key: "typography.arabic" | "typography.translation",
    current: Typography,
    limits: (typeof TYPOGRAPHY_LIMITS)["arabic" | "translation"],
  ) => (
    <div className="grid gap-4 sm:grid-cols-2">
      <RangeField
        label={t("settings.font_size")}
        value={current.size}
        min={limits.size[0]}
        max={limits.size[1]}
        step={1}
        format={pixels}
        onChange={(size) => {
          setPreference(key, { ...current, size });
        }}
      />
      <RangeField
        label={t("settings.line_height")}
        value={current.line_height}
        min={limits.line_height[0]}
        max={limits.line_height[1]}
        step={LINE_HEIGHT_STEP}
        format={lineHeight}
        onChange={(value) => {
          setPreference(key, { ...current, line_height: Math.round(value * 10) / 10 });
        }}
      />
    </div>
  );

  return (
    <div className="grid gap-6">
      <SettingsSection title={t("settings.theme")} description={t("settings.theme_hint")}>
        <SelectCard
          label={t("settings.theme_choose")}
          value={preferences["ui.theme"]}
          options={themes}
          columns={2}
          onChange={(value) => {
            setPreference("ui.theme", value);
          }}
        />
      </SettingsSection>

      <SettingsSection title={t("settings.arabic_text")}>
        <SelectCard
          label={t("settings.font")}
          value={preferences["typography.arabic_font"]}
          options={ARABIC_FONTS.map((font) => ({
            value: font,
            label: ARABIC_FONT_FACES[font].name,
            preview: (
              <span
                aria-hidden
                dir="rtl"
                lang="ar"
                className="w-14 shrink-0 text-center text-lg leading-none"
                style={{ fontFamily: ARABIC_FONT_FACES[font].family }}
              >
                {ARABIC_GLYPHS}
              </span>
            ),
          }))}
          onChange={(value) => {
            setPreference("typography.arabic_font", value);
          }}
        />
        {sizes("typography.arabic", arabic, TYPOGRAPHY_LIMITS.arabic)}
        <p
          dir="rtl"
          lang="ar"
          aria-label={t("settings.preview")}
          className="arabic-text rounded-lg bg-muted/50 px-4 text-center"
        >
          {ARABIC_SAMPLE}
        </p>
      </SettingsSection>

      <SettingsSection title={t("settings.translation_text")}>
        <SelectCard
          label={t("settings.font")}
          value={preferences["typography.translation_font"]}
          options={TRANSLATION_FONTS.map((font) => ({
            value: font,
            label: TRANSLATION_FONT_FACES[font].name || t("settings.font_system"),
            style: { fontFamily: TRANSLATION_FONT_FACES[font].family },
          }))}
          onChange={(value) => {
            setPreference("typography.translation_font", value);
          }}
        />
        {sizes("typography.translation", translation, TYPOGRAPHY_LIMITS.translation)}
        <p className="translation-text rounded-lg bg-muted/50 px-4 py-3">
          {t("settings.sample_text")}
        </p>
      </SettingsSection>

      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setPreference("typography.arabic", DEFAULT_SETTINGS["typography.arabic"]);
            setPreference("typography.translation", DEFAULT_SETTINGS["typography.translation"]);
            setPreference("typography.arabic_font", DEFAULT_SETTINGS["typography.arabic_font"]);
            setPreference(
              "typography.translation_font",
              DEFAULT_SETTINGS["typography.translation_font"],
            );
          }}
        >
          {t("settings.reset_text_sizes")}
        </Button>
      </div>
    </div>
  );
}
