import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  DEFAULT_SETTINGS,
  type ThemePreference,
  type Typography,
  TYPOGRAPHY_LIMITS,
} from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { RadioField, RangeField } from "./fields";

const THEMES: readonly ThemePreference[] = ["system", "light", "dark", "sepia"];

/** Quranic text for the preview (the basmala); content, not an interface string. */
const ARABIC_SAMPLE = "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ";

/** Line heights move in tenths. */
const LINE_HEIGHT_STEP = 0.1;

/** Theme and reader typography (spec §13.4). */
export function AppearanceSettings() {
  const { t, i18n } = useTranslation();
  const theme = useUiStore((state) => state.preferences["ui.theme"]);
  const arabic = useUiStore((state) => state.preferences["typography.arabic"]);
  const translation = useUiStore((state) => state.preferences["typography.translation"]);
  const setPreference = useUiStore((state) => state.setPreference);

  const pixels = (value: number) => t("settings.pixels", { value });
  const lineHeight = (value: number) =>
    value.toLocaleString(i18n.resolvedLanguage, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    });

  const typography = (
    key: "typography.arabic" | "typography.translation",
    current: Typography,
    limits: (typeof TYPOGRAPHY_LIMITS)["arabic" | "translation"],
  ) => (
    <>
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
    </>
  );

  return (
    <div className="grid gap-6">
      <RadioField
        legend={t("settings.theme")}
        value={theme}
        options={THEMES.map((value) => ({
          value,
          label: value === "system" ? t("settings.follow_system") : t(`settings.theme_${value}`),
        }))}
        onChange={(value) => {
          setPreference("ui.theme", value);
        }}
      />
      <fieldset className="grid gap-3">
        <legend className="mb-1 text-sm font-semibold">{t("settings.arabic_text")}</legend>
        {typography("typography.arabic", arabic, TYPOGRAPHY_LIMITS.arabic)}
        <p dir="rtl" lang="ar" className="arabic-text rounded-md border px-3">
          {ARABIC_SAMPLE}
        </p>
      </fieldset>
      <fieldset className="grid gap-3">
        <legend className="mb-1 text-sm font-semibold">{t("settings.translation_text")}</legend>
        {typography("typography.translation", translation, TYPOGRAPHY_LIMITS.translation)}
        <p className="translation-text rounded-md border px-3 py-2">{t("settings.sample_text")}</p>
      </fieldset>
      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setPreference("typography.arabic", DEFAULT_SETTINGS["typography.arabic"]);
            setPreference("typography.translation", DEFAULT_SETTINGS["typography.translation"]);
          }}
        >
          {t("settings.reset_text_sizes")}
        </Button>
      </div>
    </div>
  );
}
