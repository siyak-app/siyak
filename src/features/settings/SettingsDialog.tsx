import { SettingsIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { type LanguagePreference, type ThemePreference, useUiStore } from "@/stores/ui";

const LANGUAGES: readonly LanguagePreference[] = ["system", "tr", "en"];
const THEMES: readonly ThemePreference[] = ["system", "light", "dark"];

/**
 * Minimal settings: interface language and theme.
 * TODO(M2): appearance (fonts, sizes) and reader defaults (spec §13.3).
 */
export function SettingsDialog() {
  const { t } = useTranslation();
  const language = useUiStore((state) => state.preferences["ui.language"]);
  const theme = useUiStore((state) => state.preferences["ui.theme"]);
  const setPreference = useUiStore((state) => state.setPreference);

  const languageLabel = (value: LanguagePreference) =>
    value === "system"
      ? t("settings.follow_system")
      : t(value === "tr" ? "settings.language_tr" : "settings.language_en");
  const themeLabel = (value: ThemePreference) =>
    value === "system"
      ? t("settings.follow_system")
      : t(value === "light" ? "settings.theme_light" : "settings.theme_dark");

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon-sm" aria-label={t("layout.open_settings")}>
          <SettingsIcon />
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <label className="grid gap-1.5">
            <span className="font-medium">{t("settings.language")}</span>
            <Select
              value={language}
              onValueChange={(value) => {
                const next = LANGUAGES.find((option) => option === value);
                if (next) setPreference("ui.language", next);
              }}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LANGUAGES.map((option) => (
                  <SelectItem key={option} value={option}>
                    {languageLabel(option)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </label>
          <label className="grid gap-1.5">
            <span className="font-medium">{t("settings.theme")}</span>
            <Select
              value={theme}
              onValueChange={(value) => {
                const next = THEMES.find((option) => option === value);
                if (next) setPreference("ui.theme", next);
              }}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THEMES.map((option) => (
                  <SelectItem key={option} value={option}>
                    {themeLabel(option)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </label>
        </div>
      </DialogContent>
    </Dialog>
  );
}
