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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { LanguagePreference } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { AppearanceSettings } from "./AppearanceSettings";
import { BackupSettings } from "./BackupSettings";
import { DisplaySettings } from "./DisplaySettings";
import { RadioField } from "./fields";
import { PrivacySettings } from "./PrivacySettings";

const LANGUAGES: readonly LanguagePreference[] = ["system", "tr", "en"];

/** Settings: appearance, display & behaviour, the interface language, data and privacy. */
export function SettingsDialog() {
  const { t } = useTranslation();
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon-sm" aria-label={t("layout.open_settings")}>
          <SettingsIcon />
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[85vh] flex-col sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>
        <Tabs defaultValue="appearance" className="min-h-0 flex-1">
          <TabsList className="w-full">
            <TabsTrigger value="appearance">{t("settings.tab_appearance")}</TabsTrigger>
            <TabsTrigger value="display">{t("settings.tab_display")}</TabsTrigger>
            <TabsTrigger value="language">{t("settings.tab_language")}</TabsTrigger>
            <TabsTrigger value="data">{t("settings.tab_data")}</TabsTrigger>
            <TabsTrigger value="privacy">{t("settings.tab_privacy")}</TabsTrigger>
          </TabsList>
          <div className="-mx-1 min-h-0 overflow-y-auto px-1 pt-3">
            <TabsContent value="appearance">
              <AppearanceSettings />
            </TabsContent>
            <TabsContent value="display">
              <DisplaySettings />
            </TabsContent>
            <TabsContent value="language">
              <LanguageSettings />
            </TabsContent>
            <TabsContent value="data">
              <BackupSettings />
            </TabsContent>
            <TabsContent value="privacy">
              <PrivacySettings />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

function LanguageSettings() {
  const { t } = useTranslation();
  const language = useUiStore((state) => state.preferences["ui.language"]);
  const setPreference = useUiStore((state) => state.setPreference);
  return (
    <RadioField
      legend={t("settings.language")}
      value={language}
      options={LANGUAGES.map((value) => ({
        value,
        label: value === "system" ? t("settings.follow_system") : t(`settings.language_${value}`),
      }))}
      onChange={(value) => {
        setPreference("ui.language", value);
      }}
    />
  );
}
