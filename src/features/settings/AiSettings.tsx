import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { EmbeddingPicker } from "@/features/ai/EmbeddingPicker";
import { ModelInput, ModelSelect } from "@/features/ai/LlmPicker";
import { aiBlocked, chooseProvider, useSaveAiSetting } from "@/features/ai/models";
import { ModelKindBadge } from "@/features/ai/ModelKindBadge";
import { packTitle } from "@/features/packs/packFormat";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import type { AiStatus } from "@/lib/bindings";
import { useAiStatus, useModels, useTafsirs } from "@/lib/queries";
import { AI_PROVIDERS, type AiProvider, MAX_CHAT_TAFSIRS } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { CheckField, SelectCard, SettingsSection } from "./fields";
import { SpendingSettings } from "./SpendingSettings";

/** The value of the provider radio group that means "no AI". */
const NONE = "none";

/**
 * Settings → AI: the answer model (LLM) of chat and tafsir summaries with its
 * provider, whether it may read Arabic sources (spec §8), and apart from it the
 * search model (embedding) that finds verses by meaning (spec §7.4). Keys and
 * the server address are entered on the Providers tab.
 */
export function AiSettings() {
  const { t } = useTranslation();
  const provider = useUiStore((state) => state.preferences["ai.provider"]);
  const status = useAiStatus();
  const save = useSaveAiSetting();

  return (
    <div className="grid gap-6 text-sm">
      <p className="text-muted-foreground">{t("ai_settings.hint")}</p>
      <SettingsSection
        title={t("models.answer_model")}
        description={t("models.kind_llm_hint")}
        action={<ModelKindBadge kind="llm" />}
      >
        <SelectCard<AiProvider | typeof NONE>
          label={t("ai_settings.provider")}
          value={provider ?? NONE}
          options={[
            { value: NONE, label: t("ai_settings.provider_none") },
            ...AI_PROVIDERS.map((value) => ({ value, label: PROVIDER_NAMES[value] })),
          ]}
          onChange={(value) => {
            chooseProvider(save, provider, value === NONE ? null : value);
          }}
        />
        {status.data && <StatusNote status={status.data} />}
        {provider && <Models provider={provider} />}
        {provider && <ChatTafsirs />}
        {save.error && <ErrorMessage error={save.error} />}
      </SettingsSection>
      <EmbeddingPicker />
      <SpendingSettings />
    </div>
  );
}

/** What still has to be done before AI features work. */
function StatusNote({ status }: { status: AiStatus }) {
  const { t } = useTranslation();
  const setTab = useUiStore((state) => state.setSettingsTab);
  const toProviders = (
    <Button
      variant="link"
      size="sm"
      className="h-auto p-0"
      onClick={() => {
        setTab("providers");
      }}
    >
      {t("ai_settings.open_providers")}
    </Button>
  );
  switch (status.state) {
    case "key_missing":
      return (
        <p className="text-muted-foreground">
          {t("ai.key_missing", { provider: PROVIDER_NAMES[status.provider] })} {toProviders}
        </p>
      );
    case "base_url_missing":
      return (
        <p className="text-muted-foreground">
          {t("ai.base_url_missing")} {toProviders}
        </p>
      );
    case "base_url_invalid":
      return <p className="text-destructive">{t("semantic.base_url_invalid")}</p>;
    case "ready":
      return (
        <p className="text-muted-foreground" role="status">
          {t("ai_settings.ready")}
        </p>
      );
    case "no_provider":
    case "no_model":
      return null;
  }
}

function Models({ provider }: { provider: AiProvider }) {
  const { t } = useTranslation();
  const status = useAiStatus();
  const chat = useUiStore((state) => state.preferences["ai.chat_model"]);
  const translation = useUiStore((state) => state.preferences["ai.translation_model"]);
  const arabic = useUiStore((state) => state.preferences["ai.arabic_sources"]);
  const save = useSaveAiSetting();
  const blocked = aiBlocked(status.data);
  const models = useModels(provider, !blocked);
  const ids = models.data?.map((model) => model.id) ?? [];
  const names = new Map(models.data?.map((model) => [model.id, model.name ?? model.id]));
  const manual = models.isError || (models.isSuccess && ids.length === 0);
  const arabicKey = chat ? `${provider}/${chat}` : null;
  // Off by default for a local server, which usually runs a small model (spec §8).
  const arabicDefault = provider !== "openai_compatible";

  if (blocked) return null;
  return (
    <>
      {models.isError && (
        <div className="grid gap-1">
          <p className="text-muted-foreground">{t("ai_settings.models_failed")}</p>
          <ErrorMessage error={models.error} />
        </div>
      )}
      {manual ? (
        <>
          <ModelInput
            label={t("ai_settings.chat_model")}
            value={chat}
            onSave={(value) => {
              save("ai.chat_model", value);
            }}
          />
          <ModelInput
            label={t("ai_settings.translation_model")}
            hint={t("ai_settings.translation_model_hint")}
            value={translation}
            onSave={(value) => {
              save("ai.translation_model", value);
            }}
          />
        </>
      ) : (
        <>
          <ModelSelect
            label={t("ai_settings.chat_model")}
            value={chat}
            ids={ids}
            names={names}
            loading={models.isPending}
            onChange={(value) => {
              save("ai.chat_model", value);
            }}
          />
          <ModelSelect
            label={t("ai_settings.translation_model")}
            hint={t("ai_settings.translation_model_hint")}
            value={translation}
            ids={ids}
            names={names}
            loading={models.isPending}
            sameAsChat
            chatModel={chat}
            onChange={(value) => {
              save("ai.translation_model", value);
            }}
          />
        </>
      )}
      {arabicKey && (
        <CheckField
          label={t("ai_settings.arabic_sources")}
          hint={t("ai_settings.arabic_sources_hint")}
          checked={arabic[arabicKey] ?? arabicDefault}
          onChange={(checked) => {
            save("ai.arabic_sources", { ...arabic, [arabicKey]: checked });
          }}
        />
      )}
    </>
  );
}

/** Which installed tafsirs go into chat answers, at most three (spec §10). */
function ChatTafsirs() {
  const { t, i18n } = useTranslation();
  const tafsirs = useTafsirs();
  const chosen = useUiStore((state) => state.preferences["chat.tafsirs"]);
  const save = useSaveAiSetting();
  if (!tafsirs.data || tafsirs.data.length === 0) return null;
  const full = chosen.length >= MAX_CHAT_TAFSIRS;
  return (
    <fieldset className="grid gap-1.5">
      <legend className="mb-1 font-medium">{t("ai_settings.chat_tafsirs")}</legend>
      {tafsirs.data.map((tafsir) => {
        const checked = chosen.includes(tafsir.id);
        return (
          <CheckField
            key={tafsir.id}
            label={packTitle(tafsir.titles, tafsir.title, i18n.resolvedLanguage)}
            checked={checked}
            disabled={!checked && full}
            onChange={(on) => {
              save(
                "chat.tafsirs",
                on ? [...chosen, tafsir.id] : chosen.filter((id) => id !== tafsir.id),
              );
            }}
          />
        );
      })}
      <p className="text-xs text-muted-foreground">
        {t("ai_settings.chat_tafsirs_hint", { count: MAX_CHAT_TAFSIRS })}
      </p>
    </fieldset>
  );
}
