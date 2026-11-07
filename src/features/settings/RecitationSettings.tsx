import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { reciterTitle, useReciter } from "@/features/audio/useReciter";
import { useUiStore } from "@/stores/ui";

import { CheckField, RadioField } from "./fields";

/**
 * Recitation (ADR 0014): the default reciter among the installed audio packs,
 * continuing with the next verse and following the recited verse. Without a
 * reciter it points to the Packs screen.
 */
export function RecitationSettings() {
  const { t, i18n } = useTranslation();
  const preferences = useUiStore((state) => state.preferences);
  const setPreference = useUiStore((state) => state.setPreference);
  const setPacksDialogOpen = useUiStore((state) => state.setPacksDialogOpen);
  const setSettingsTab = useUiStore((state) => state.setSettingsTab);
  const { packs, active } = useReciter();

  return (
    <section className="grid gap-3" aria-labelledby="settings-recitation-heading">
      <h3 id="settings-recitation-heading" className="text-sm font-semibold">
        {t("settings.recitation")}
      </h3>
      <p className="text-xs text-muted-foreground">{t("settings.recitation_hint")}</p>
      {active ? (
        <RadioField
          legend={t("settings.recitation_reciter")}
          value={active.id}
          options={packs.map((pack) => ({
            value: pack.id,
            label: reciterTitle(pack, i18n.resolvedLanguage),
          }))}
          onChange={(id) => {
            setPreference("audio.reciter", id);
          }}
        />
      ) : (
        <div className="flex flex-wrap items-center gap-2">
          <p className="text-sm text-muted-foreground">{t("settings.recitation_none")}</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setSettingsTab(null);
              setPacksDialogOpen(true);
            }}
          >
            {t("settings.recitation_get")}
          </Button>
        </div>
      )}
      <CheckField
        label={t("settings.recitation_autoplay")}
        checked={preferences["audio.autoplay"]}
        onChange={(value) => {
          setPreference("audio.autoplay", value);
        }}
      />
      <CheckField
        label={t("settings.recitation_follow")}
        checked={preferences["audio.follow"]}
        onChange={(value) => {
          setPreference("audio.follow", value);
        }}
      />
    </section>
  );
}
