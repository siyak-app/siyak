import { type SyntheticEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { packTitle } from "@/features/packs/packFormat";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import type { ApiProvider } from "@/lib/bindings";
import {
  useApiKeys,
  useCatalog,
  useDeleteApiKey,
  useSemanticStatus,
  useSetApiKey,
  useSetSemanticSetting,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { RadioField } from "./fields";

const PROVIDERS: readonly ApiProvider[] = ["openai", "voyage", "gemini", "openai_compatible"];

/**
 * Settings → Providers: API keys in the OS keychain, the address of an
 * OpenAI-compatible server and the active semantic search pack (spec §7.4,
 * §14.2). Keys never come back from the core; the screen only shows whether
 * each one is saved.
 */
export function ProviderSettings() {
  const { t } = useTranslation();
  const keys = useApiKeys();
  return (
    <div className="grid gap-6 text-sm">
      <section className="grid gap-2">
        <h3 className="font-medium">{t("settings.providers_title")}</h3>
        <p className="text-muted-foreground">{t("settings.providers_hint")}</p>
        <ul className="list-disc ps-5 text-xs text-muted-foreground">
          <li>{t("settings.key_advice_separate")}</li>
          <li>{t("settings.key_advice_limit")}</li>
          <li>{t("settings.key_advice_only_provider")}</li>
        </ul>
      </section>
      {keys.isError && <ErrorMessage error={keys.error} />}
      {PROVIDERS.map((provider) => (
        <KeyField
          key={provider}
          provider={provider}
          saved={keys.data?.find((status) => status.provider === provider)?.saved}
        />
      ))}
      <ServerAddress />
      <ActiveVectorPack />
    </div>
  );
}

function KeyField({ provider, saved }: { provider: ApiProvider; saved: boolean | undefined }) {
  const { t } = useTranslation();
  const inputId = useId();
  const [value, setValue] = useState("");
  const save = useSetApiKey();
  const remove = useDeleteApiKey();
  const name = PROVIDER_NAMES[provider];
  const optional = provider === "openai_compatible";

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    if (!value.trim()) return;
    save.mutate(
      { provider, key: value },
      {
        onSuccess: () => {
          setValue("");
        },
      },
    );
  };

  return (
    <section className="grid gap-2" aria-labelledby={`${inputId}-title`}>
      <div className="flex items-baseline justify-between gap-2">
        <h4 id={`${inputId}-title`} className="font-medium">
          {name}
          {optional && (
            <span className="ms-1 font-normal text-muted-foreground">
              {t("settings.key_optional")}
            </span>
          )}
        </h4>
        {saved !== undefined && (
          <span className="text-xs text-muted-foreground" role="status">
            {saved ? t("settings.key_saved") : t("settings.key_not_saved")}
          </span>
        )}
      </div>
      <form className="flex gap-2" onSubmit={submit}>
        <label htmlFor={inputId} className="sr-only">
          {t("settings.key_label", { provider: name })}
        </label>
        <Input
          id={inputId}
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={value}
          placeholder={saved ? t("settings.key_replace_placeholder") : undefined}
          onChange={(event) => {
            setValue(event.target.value);
          }}
        />
        <Button
          type="submit"
          variant="outline"
          size="sm"
          disabled={!value.trim() || save.isPending}
        >
          {t("settings.key_save")}
        </Button>
        {saved && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={remove.isPending}
            onClick={() => {
              remove.mutate(provider);
            }}
          >
            {t("settings.key_remove")}
          </Button>
        )}
      </form>
      {save.isError && <ErrorMessage error={save.error} />}
      {remove.isError && <ErrorMessage error={remove.error} />}
    </section>
  );
}

function ServerAddress() {
  const { t } = useTranslation();
  const inputId = useId();
  const hintId = useId();
  const stored = useUiStore((state) => state.preferences["semantic.base_url"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const [value, setValue] = useState(stored ?? "");
  const save = useSetSemanticSetting();
  const status = useSemanticStatus();
  const changed = value.trim() !== (stored ?? "");

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    const address = value.trim() || null;
    save.mutate(
      { key: "semantic.base_url", value: address },
      {
        onSuccess: () => {
          setPreference("semantic.base_url", address);
        },
      },
    );
  };

  return (
    <section className="grid gap-2">
      <label htmlFor={inputId} className="font-medium">
        {t("settings.server_label")}
      </label>
      <form className="flex gap-2" onSubmit={submit}>
        <Input
          id={inputId}
          type="url"
          inputMode="url"
          autoComplete="off"
          spellCheck={false}
          placeholder={t("settings.server_placeholder")}
          aria-describedby={hintId}
          value={value}
          onChange={(event) => {
            setValue(event.target.value);
          }}
        />
        <Button type="submit" variant="outline" size="sm" disabled={!changed || save.isPending}>
          {t("settings.server_save")}
        </Button>
      </form>
      <p id={hintId} className="text-xs text-muted-foreground">
        {t("settings.server_hint")}
      </p>
      {status.data?.state === "base_url_invalid" && !changed && (
        <p className="text-xs text-destructive">{t("semantic.base_url_invalid")}</p>
      )}
      {save.isError && <ErrorMessage error={save.error} />}
    </section>
  );
}

/** Which installed vector pack semantic search uses, when there is a choice. */
function ActiveVectorPack() {
  const { t, i18n } = useTranslation();
  const catalog = useCatalog();
  const chosen = useUiStore((state) => state.preferences["semantic.vector_pack"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const save = useSetSemanticSetting();
  const vectors =
    catalog.data?.packs.filter((pack) => pack.kind === "vectors" && pack.installed) ?? [];
  const first = vectors[0];
  if (!first || vectors.length < 2) return null;
  const active = vectors.find((pack) => pack.id === chosen) ?? first;
  return (
    <section className="grid gap-2">
      <RadioField
        legend={t("settings.active_pack")}
        hint={t("settings.active_pack_hint")}
        value={active.id}
        options={vectors.map((pack) => ({
          value: pack.id,
          label: packTitle(pack.title, pack.title_fallback, i18n.resolvedLanguage),
        }))}
        onChange={(id) => {
          save.mutate(
            { key: "semantic.vector_pack", value: id },
            {
              onSuccess: () => {
                setPreference("semantic.vector_pack", id);
              },
            },
          );
        }}
      />
      {save.isError && <ErrorMessage error={save.error} />}
    </section>
  );
}
