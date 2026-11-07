import { type SyntheticEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import type { AiStatus } from "@/lib/bindings";
import { packTitle } from "@/features/packs/packFormat";
import {
  type AiSettingKey,
  useAiStatus,
  useModels,
  useSetAiSetting,
  useTafsirs,
} from "@/lib/queries";
import { AI_PROVIDERS, type AiProvider, MAX_CHAT_TAFSIRS, type Settings } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { CheckField, RadioField } from "./fields";
import { SpendingSettings } from "./SpendingSettings";

/** The value of the provider radio group that means "no AI". */
const NONE = "none";

/** The model select's value for "same as the chat model". */
const SAME_AS_CHAT = "__same__";

/**
 * Settings → AI: the provider of chat and tafsir summaries, the model of each
 * role and whether the chat model may read Arabic sources (spec §8). Keys and
 * the server address are entered on the Providers tab.
 */
export function AiSettings() {
  const { t } = useTranslation();
  const provider = useUiStore((state) => state.preferences["ai.provider"]);
  const status = useAiStatus();
  const save = useSaveAiSetting();

  return (
    <div className="grid gap-6 text-sm">
      <section className="grid gap-2">
        <h3 className="font-medium">{t("ai_settings.title")}</h3>
        <p className="text-muted-foreground">{t("ai_settings.hint")}</p>
      </section>
      <RadioField<AiProvider | typeof NONE>
        legend={t("ai_settings.provider")}
        value={provider ?? NONE}
        options={[
          { value: NONE, label: t("ai_settings.provider_none") },
          ...AI_PROVIDERS.map((value) => ({ value, label: PROVIDER_NAMES[value] })),
        ]}
        onChange={(value) => {
          const next = value === NONE ? null : value;
          if (next === provider) return;
          save("ai.provider", next);
          // Models belong to a provider.
          save("ai.chat_model", null);
          save("ai.translation_model", null);
        }}
      />
      {status.data && <StatusNote status={status.data} />}
      {provider && <Models provider={provider} />}
      {provider && <ChatTafsirs />}
      {save.error && <ErrorMessage error={save.error} />}
      <SpendingSettings />
    </div>
  );
}

/** Saves an AI setting and mirrors it in the UI store. */
function useSaveAiSetting() {
  const setPreference = useUiStore((state) => state.setPreference);
  const mutation = useSetAiSetting();
  const save = <K extends AiSettingKey>(key: K, value: Settings[K]) => {
    setPreference(key, value);
    mutation.mutate({ key, value });
  };
  return Object.assign(save, { error: mutation.isError ? mutation.error : null });
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
  const blocked =
    status.data?.state === "key_missing" ||
    status.data?.state === "base_url_missing" ||
    status.data?.state === "base_url_invalid";
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

function ModelSelect({
  label,
  hint,
  value,
  ids,
  names,
  loading,
  sameAsChat = false,
  onChange,
}: {
  label: string;
  hint?: string;
  value: string | null;
  ids: string[];
  names: Map<string, string>;
  loading: boolean;
  sameAsChat?: boolean;
  onChange: (value: string | null) => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const hintId = useId();
  // A chosen model the provider no longer lists stays selectable.
  const options = value && !ids.includes(value) ? [value, ...ids] : ids;
  return (
    <div className="grid gap-1.5">
      <label htmlFor={id} className="font-medium">
        {label}
      </label>
      <Select
        value={value ?? (sameAsChat ? SAME_AS_CHAT : "")}
        disabled={loading}
        onValueChange={(next) => {
          onChange(next === SAME_AS_CHAT ? null : next);
        }}
      >
        <SelectTrigger id={id} className="w-full" aria-describedby={hint ? hintId : undefined}>
          <SelectValue
            placeholder={loading ? t("ai_settings.models_loading") : t("ai_settings.choose_model")}
          />
        </SelectTrigger>
        <SelectContent>
          {sameAsChat && (
            <SelectItem value={SAME_AS_CHAT}>{t("ai_settings.same_as_chat")}</SelectItem>
          )}
          {options.map((option) => (
            <SelectItem key={option} value={option}>
              {names.get(option) ?? option}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {hint && (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
    </div>
  );
}

/** A model id typed by hand, when the provider's list cannot be loaded. */
function ModelInput({
  label,
  hint,
  value,
  onSave,
}: {
  label: string;
  hint?: string;
  value: string | null;
  onSave: (value: string | null) => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const hintId = useId();
  const [text, setText] = useState(value ?? "");
  const changed = text.trim() !== (value ?? "");
  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    onSave(text.trim() || null);
  };
  return (
    <form className="grid gap-1.5" onSubmit={submit}>
      <label htmlFor={id} className="font-medium">
        {label}
      </label>
      <div className="flex gap-2">
        <Input
          id={id}
          autoComplete="off"
          spellCheck={false}
          aria-describedby={hint ? hintId : undefined}
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
        />
        <Button type="submit" variant="outline" size="sm" disabled={!changed}>
          {t("settings.key_save")}
        </Button>
      </div>
      {hint && (
        <p id={hintId} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
    </form>
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
