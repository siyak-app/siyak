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
import { SETTINGS_TABS, type SettingsTab, useUiStore } from "@/stores/ui";

import { AiSettings } from "./AiSettings";
import { AppearanceSettings } from "./AppearanceSettings";
import { BackupSettings } from "./BackupSettings";
import { DisplaySettings } from "./DisplaySettings";
import { RadioField } from "./fields";
import { PrivacySettings } from "./PrivacySettings";
import { ProviderSettings } from "./ProviderSettings";

const LANGUAGES: readonly LanguagePreference[] = ["system", "tr", "en"];

const isTab = (value: string): value is SettingsTab =>
  (SETTINGS_TABS as readonly string[]).includes(value);

/**
 * Settings: appearance, display & behaviour, the interface language, API
 * providers, AI, data and privacy. Other screens open it on a tab through the UI
 * store (`setSettingsTab`).
 */
export function SettingsDialog() {
  const { t } = useTranslation();
  const tab = useUiStore((state) => state.settingsTab);
  const setTab = useUiStore((state) => state.setSettingsTab);
  return (
    <Dialog
      open={tab !== null}
      onOpenChange={(open) => {
        setTab(open ? "appearance" : null);
      }}
    >
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
        <Tabs
          value={tab ?? "appearance"}
          onValueChange={(value) => {
            if (isTab(value)) setTab(value);
          }}
          className="min-h-0 flex-1"
        >
          <TabsList className="w-full">
            <TabsTrigger value="appearance">{t("settings.tab_appearance")}</TabsTrigger>
            <TabsTrigger value="display">{t("settings.tab_display")}</TabsTrigger>
            <TabsTrigger value="language">{t("settings.tab_language")}</TabsTrigger>
            <TabsTrigger value="providers">{t("settings.tab_providers")}</TabsTrigger>
            <TabsTrigger value="ai">{t("settings.tab_ai")}</TabsTrigger>
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
            <TabsContent value="providers">
              <ProviderSettings />
            </TabsContent>
            <TabsContent value="ai">
              <AiSettings />
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
