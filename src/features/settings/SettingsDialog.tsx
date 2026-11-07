import {
  BookOpenTextIcon,
  DatabaseIcon,
  KeyRoundIcon,
  LanguagesIcon,
  type LucideIcon,
  PaletteIcon,
  SettingsIcon,
  ShieldIcon,
  SparklesIcon,
} from "lucide-react";
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
import { SelectCard, SettingsSection } from "./fields";
import { PrivacySettings } from "./PrivacySettings";
import { ProviderSettings } from "./ProviderSettings";

const LANGUAGES: readonly LanguagePreference[] = ["system", "tr", "en"];

/** The icon of each tab in the side menu. */
const TAB_ICONS: Record<SettingsTab, LucideIcon> = {
  appearance: PaletteIcon,
  display: BookOpenTextIcon,
  language: LanguagesIcon,
  providers: KeyRoundIcon,
  ai: SparklesIcon,
  data: DatabaseIcon,
  privacy: ShieldIcon,
};

const isTab = (value: string): value is SettingsTab =>
  (SETTINGS_TABS as readonly string[]).includes(value);

/**
 * Settings: appearance, reading, the interface language, API providers, AI,
 * data and privacy, chosen from a side menu. Other screens open it on a tab
 * through the UI store (`setSettingsTab`).
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
      <DialogContent className="flex h-[min(85vh,760px)] flex-col gap-4 sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>
        <Tabs
          orientation="vertical"
          value={tab ?? "appearance"}
          onValueChange={(value) => {
            if (isTab(value)) setTab(value);
          }}
          className="min-h-0 flex-1 gap-4"
        >
          <TabsList className="w-44 shrink-0 items-stretch gap-0.5 bg-transparent p-0">
            {SETTINGS_TABS.map((value) => {
              const Icon = TAB_ICONS[value];
              return (
                <TabsTrigger
                  key={value}
                  value={value}
                  className="h-9 flex-none justify-start gap-2.5 px-2.5 data-active:bg-primary/10 data-active:text-primary data-active:shadow-none dark:data-active:border-transparent dark:data-active:bg-primary/15"
                >
                  <Icon aria-hidden />
                  {t(`settings.tab_${value}`)}
                </TabsTrigger>
              );
            })}
          </TabsList>
          <div className="-my-1 min-h-0 flex-1 overflow-y-auto border-s py-1 ps-5 pe-1">
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
    <SettingsSection title={t("settings.language")} description={t("settings.language_hint")}>
      <SelectCard
        label={t("settings.language_choose")}
        value={language}
        options={LANGUAGES.map((value) => ({
          value,
          label: value === "system" ? t("settings.follow_system") : t(`settings.language_${value}`),
        }))}
        onChange={(value) => {
          setPreference("ui.language", value);
        }}
      />
    </SettingsSection>
  );
}
