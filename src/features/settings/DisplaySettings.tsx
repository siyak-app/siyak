import { useTranslation } from "react-i18next";

import { Term } from "@/components/Term";
import { TranslationChooser } from "@/features/reader/TranslationPicker";
import { useCitationStyles } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { SelectCard, SettingsSection } from "./fields";
import { RecitationSettings } from "./RecitationSettings";

/**
 * Reading: the translations shown, recitation, how technical terms read and
 * the citation style. Everything that differs between reading and research
 * mode follows the mode switch in the header (spec §13.2) and is not
 * repeated here.
 */
export function DisplaySettings() {
  const { t } = useTranslation();
  const terms = useUiStore((state) => state.preferences["ui.technical_terms"]);
  const setPreference = useUiStore((state) => state.setPreference);

  return (
    <div className="grid gap-6">
      <SettingsSection
        title={t("reader.translations_picker_title")}
        description={t("reader.translations_picker_hint")}
      >
        <TranslationChooser />
      </SettingsSection>

      <RecitationSettings />

      <SettingsSection title={t("settings.terms_and_citations")}>
        <SelectCard
          label={t("settings.technical_terms")}
          value={terms}
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
        <CitationStyleField />
      </SettingsSection>
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
    <SelectCard
      label={t("settings.citation_style")}
      value={style}
      options={styles.data.map(({ id, name }) => ({ value: id, label: name }))}
      onChange={(value) => {
        setPreference("reader.citation_style", value);
      }}
    />
  );
}
