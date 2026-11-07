import { cn } from "cn";
import { type SyntheticEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

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
import { useAiStatus, useModels } from "@/lib/queries";
import { AI_PROVIDERS, type AiProvider } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { ModelKindBadge } from "./ModelKindBadge";
import { aiBlocked, chooseProvider, useSaveAiSetting } from "./models";

/** The model select's value for "same as the chat model". */
const SAME_AS_CHAT = "__same__";

/**
 * The model of an AI action as a compact control above it: the provider and
 * its model, saved to the same settings as Settings → AI. `chat` is the model
 * of the Chat and Find screens; `translation` the one of tafsir summaries and
 * translations, which may follow the chat model. What is still missing (a
 * key, an address) is said by the screen below it.
 */
export function LlmPicker({ role = "chat" }: { role?: "chat" | "translation" }) {
  const { t } = useTranslation();
  const provider = useUiStore((state) => state.preferences["ai.provider"]);
  const chat = useUiStore((state) => state.preferences["ai.chat_model"]);
  const translation = useUiStore((state) => state.preferences["ai.translation_model"]);
  const status = useAiStatus();
  const save = useSaveAiSetting();
  const blocked = aiBlocked(status.data);
  const models = useModels(provider, !blocked);
  const ids = models.data?.map((model) => model.id) ?? [];
  const names = new Map(models.data?.map((model) => [model.id, model.name ?? model.id]));
  const manual = models.isError || (models.isSuccess && ids.length === 0);
  const providerId = useId();
  const key = role === "chat" ? "ai.chat_model" : "ai.translation_model";
  const label = t(role === "chat" ? "ai_settings.chat_model" : "ai_settings.translation_model");
  const value = role === "chat" ? chat : translation;

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <ModelKindBadge kind="llm" />
      <label htmlFor={providerId} className="sr-only">
        {t("ai_settings.provider")}
      </label>
      <Select
        value={provider ?? ""}
        onValueChange={(next) => {
          chooseProvider(save, provider, next as AiProvider);
        }}
      >
        <SelectTrigger id={providerId} size="sm" className="max-w-48">
          <SelectValue placeholder={t("models.choose_provider")} />
        </SelectTrigger>
        <SelectContent>
          {AI_PROVIDERS.map((value) => (
            <SelectItem key={value} value={value}>
              {PROVIDER_NAMES[value]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {provider &&
        !blocked &&
        (manual ? (
          <ModelInput
            compact
            label={label}
            value={value}
            onSave={(next) => {
              save(key, next);
            }}
          />
        ) : (
          <ModelSelect
            compact
            label={label}
            value={value}
            ids={ids}
            names={names}
            loading={models.isPending}
            sameAsChat={role === "translation"}
            chatModel={chat}
            onChange={(next) => {
              save(key, next);
            }}
          />
        ))}
    </div>
  );
}

/** A model out of the provider's list; `compact` hides the label for toolbars. */
export function ModelSelect({
  label,
  hint,
  value,
  ids,
  names,
  loading,
  sameAsChat = false,
  chatModel = null,
  compact = false,
  onChange,
}: {
  label: string;
  hint?: string;
  value: string | null;
  ids: string[];
  names: Map<string, string>;
  loading: boolean;
  sameAsChat?: boolean;
  /** The chat model, named in the "same as the chat model" option. */
  chatModel?: string | null;
  compact?: boolean;
  onChange: (value: string | null) => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const hintId = useId();
  // A chosen model the provider no longer lists stays selectable.
  const options = value && !ids.includes(value) ? [value, ...ids] : ids;
  return (
    <div className={compact ? "contents" : "grid gap-1.5"}>
      <label htmlFor={id} className={compact ? "sr-only" : "font-medium"}>
        {label}
      </label>
      <Select
        value={value ?? (sameAsChat ? SAME_AS_CHAT : "")}
        disabled={loading}
        onValueChange={(next) => {
          onChange(next === SAME_AS_CHAT ? null : next);
        }}
      >
        <SelectTrigger
          id={id}
          size={compact ? "sm" : "default"}
          className={compact ? "max-w-64" : "w-full"}
          aria-describedby={hint ? hintId : undefined}
        >
          <SelectValue
            placeholder={loading ? t("ai_settings.models_loading") : t("ai_settings.choose_model")}
          />
        </SelectTrigger>
        <SelectContent>
          {sameAsChat && (
            <SelectItem value={SAME_AS_CHAT}>
              {chatModel
                ? t("ai_settings.same_as_chat_model", {
                    model: names.get(chatModel) ?? chatModel,
                  })
                : t("ai_settings.same_as_chat")}
            </SelectItem>
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
export function ModelInput({
  label,
  hint,
  value,
  compact = false,
  onSave,
}: {
  label: string;
  hint?: string;
  value: string | null;
  compact?: boolean;
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
    <form className={compact ? "flex items-center gap-1" : "grid gap-1.5"} onSubmit={submit}>
      <label htmlFor={id} className={compact ? "sr-only" : "font-medium"}>
        {label}
      </label>
      <div className="flex gap-2">
        <Input
          id={id}
          autoComplete="off"
          spellCheck={false}
          placeholder={compact ? label : undefined}
          aria-describedby={hint ? hintId : undefined}
          className={cn(compact && "h-7 w-48")}
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
