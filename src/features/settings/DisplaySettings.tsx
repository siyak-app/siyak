import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Term } from "@/components/Term";
import { Button } from "@/components/ui/button";
import { useApplyProfile } from "@/features/profile/useApplyProfile";
import { TranslationChooser } from "@/features/reader/TranslationPicker";
import { useCitationStyles } from "@/lib/queries";
import { CONTEXT_TABS, type ExperienceProfile } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { CheckField, RadioField } from "./fields";

const PROFILES: readonly ExperienceProfile[] = ["reader", "researcher"];

/**
 * Display & behaviour: every setting an experience profile sets (spec §13.2),
 * each adjustable on its own, and a reset to either profile's defaults.
 * Settings for features of later milestones are shown but disabled.
 */
export function DisplaySettings() {
  const { t } = useTranslation();
  const preferences = useUiStore((state) => state.preferences);
  const setPreference = useUiStore((state) => state.setPreference);
  const { apply, busy, error } = useApplyProfile();
  const panel = preferences["layout.right_panel"];
  const profile = preferences.experience_profile;

  return (
    <div className="grid gap-6">
      <section className="grid gap-2" aria-labelledby="settings-profile-heading">
        <h3 id="settings-profile-heading" className="text-sm font-semibold">
          {t("settings.profile")}
        </h3>
        <p className="text-sm text-muted-foreground">
          {profile
            ? t("settings.profile_current", { profile: t(`onboarding.${profile}_title`) })
            : t("settings.profile_none")}
        </p>
        <div className="flex flex-wrap gap-2">
          {PROFILES.map((option) => (
            <Button
              key={option}
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={() => {
                apply(option);
              }}
            >
              {t("settings.profile_reset", { profile: t(`onboarding.${option}_title`) })}
            </Button>
          ))}
        </div>
        <p className="text-xs text-muted-foreground">{t("settings.profile_reset_hint")}</p>
        {error && <ErrorMessage error={error} />}
      </section>

      <section className="grid gap-2" aria-labelledby="settings-translations-heading">
        <h3 id="settings-translations-heading" className="text-sm font-semibold">
          {t("reader.translations_picker_title")}
        </h3>
        <p className="text-xs text-muted-foreground">{t("reader.translations_picker_hint")}</p>
        <TranslationChooser />
      </section>

      <RadioField
        legend={t("settings.verse_actions")}
        value={preferences["reader.actions"]}
        options={[
          { value: "hover", label: t("settings.verse_actions_hover") },
          { value: "labeled", label: t("settings.verse_actions_labeled") },
        ]}
        onChange={(value) => {
          setPreference("reader.actions", value);
        }}
      />

      <div className="grid gap-3">
        <CheckField
          label={t("settings.side_panel_open")}
          checked={!panel.collapsed}
          onChange={(open) => {
            setPreference("layout.right_panel", { ...panel, collapsed: !open });
          }}
        />
        <RadioField
          legend={t("settings.side_panel_tab")}
          value={panel.tab}
          options={CONTEXT_TABS.map((tab) => ({
            value: tab,
            label: t(`context_panel.tabs.${tab}`),
          }))}
          onChange={(tab) => {
            setPreference("layout.right_panel", { ...panel, tab });
          }}
        />
      </div>

      <RadioField
        legend={t("settings.technical_terms")}
        value={preferences["ui.technical_terms"]}
        options={[
          { value: "tooltip", label: t("settings.technical_terms_tooltip") },
          { value: "inline", label: t("settings.technical_terms_inline") },
        ]}
        onChange={(value) => {
          setPreference("ui.technical_terms", value);
        }}
        hint={
          <>
            {t("settings.example")} <Term name="lemma" />, <Term name="morphology" />
          </>
        }
      />

      <RadioField
        legend={t("settings.word_click")}
        value={preferences["word.click"]}
        options={[
          { value: "card", label: t("settings.word_click_card") },
          { value: "panel", label: t("settings.word_click_panel") },
        ]}
        onChange={(value) => {
          setPreference("word.click", value);
        }}
        unavailable
      />
      <RadioField
        legend={t("settings.word_underline")}
        value={preferences["word.underline"]}
        options={[
          { value: "none", label: t("settings.word_underline_none") },
          { value: "dotted", label: t("settings.word_underline_dotted") },
        ]}
        onChange={(value) => {
          setPreference("word.underline", value);
        }}
        unavailable
      />
      <div className="grid gap-3">
        <CheckField
          label={t("settings.search_mode_chips")}
          checked={preferences["search.mode_chips"]}
          onChange={(value) => {
            setPreference("search.mode_chips", value);
          }}
        />
        <CheckField
          label={t("settings.citation_action")}
          checked={preferences["reader.citation_action"]}
          onChange={(value) => {
            setPreference("reader.citation_action", value);
          }}
        />
        {preferences["reader.citation_action"] && <CitationStyleField />}
      </div>
    </div>
  );
}

/** The style of copied citations and of bibliographies. */
function CitationStyleField() {
  const { t } = useTranslation();
  const style = useUiStore((state) => state.preferences["reader.citation_style"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const styles = useCitationStyles();
  if (!styles.data) return null;
  return (
    <RadioField
      legend={t("settings.citation_style")}
      value={style}
      options={styles.data.map(({ id, name }) => ({ value: id, label: name }))}
      onChange={(value) => {
        setPreference("reader.citation_style", value);
      }}
    />
  );
}
