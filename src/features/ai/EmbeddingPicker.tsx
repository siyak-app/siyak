import { DownloadIcon } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { packTitle } from "@/features/packs/packFormat";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import { RadioField } from "@/features/settings/fields";
import type { PackView } from "@/lib/bindings";
import { useCatalog, useSetSemanticSetting } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { ModelKindBadge } from "./ModelKindBadge";

/**
 * The embedding model that finds verses by meaning (spec §7.4): one of the
 * installed vector packs, named by who embeds the queries. With none
 * installed, a button opens the Packs dialog to download one. `compact` is
 * the one-line form above the Chat and Find screens.
 */
export function EmbeddingPicker({ compact = false }: { compact?: boolean }) {
  const { t, i18n } = useTranslation();
  const catalog = useCatalog();
  const chosen = useUiStore((state) => state.preferences["semantic.vector_pack"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const setPacksDialogOpen = useUiStore((state) => state.setPacksDialogOpen);
  const save = useSetSemanticSetting();
  const selectId = useId();
  const language = i18n.resolvedLanguage;

  const vectors = catalog.data?.packs.filter((pack) => pack.kind === "vectors") ?? [];
  const installed = vectors.filter((pack) => pack.installed);
  const more = vectors.some((pack) => !pack.installed && pack.supported && !pack.incompatible);
  const active = installed.find((pack) => pack.id === chosen) ?? installed[0];
  const label = (pack: PackView) => embeddingName(pack, t("models.on_device"), language);

  const choose = (id: string) => {
    save.mutate(
      { key: "semantic.vector_pack", value: id },
      {
        onSuccess: () => {
          setPreference("semantic.vector_pack", id);
        },
      },
    );
  };
  const download = (
    <Button
      variant={installed.length === 0 ? "outline" : "link"}
      size="sm"
      className={installed.length === 0 ? undefined : "h-auto p-0"}
      onClick={() => {
        setPacksDialogOpen(true, "vectors");
      }}
    >
      {installed.length === 0 && <DownloadIcon aria-hidden />}
      {installed.length === 0 ? t("models.download_embedding") : t("models.download_another")}
    </Button>
  );

  if (catalog.isPending) return null;

  if (compact) {
    return (
      <div className="flex flex-wrap items-center gap-1.5">
        <ModelKindBadge kind="embedding" />
        {active ? (
          <>
            <label htmlFor={selectId} className="sr-only">
              {t("models.embedding_model")}
            </label>
            <Select value={active.id} onValueChange={choose}>
              <SelectTrigger id={selectId} size="sm" className="max-w-64">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {installed.map((pack) => (
                  <SelectItem key={pack.id} value={pack.id}>
                    {label(pack)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </>
        ) : (
          <span className="text-xs text-muted-foreground">{t("models.no_embedding")}</span>
        )}
        {(installed.length === 0 || more) && download}
        {save.isError && <ErrorMessage error={save.error} />}
      </div>
    );
  }

  return (
    <section className="grid gap-2">
      <div className="flex items-center gap-2">
        <h4 className="font-medium">{t("models.embedding_model")}</h4>
        <ModelKindBadge kind="embedding" />
      </div>
      <p className="text-xs text-muted-foreground">{t("models.embedding_hint")}</p>
      {catalog.isError && <ErrorMessage error={catalog.error} />}
      {active ? (
        <RadioField
          legend={t("models.installed")}
          hint={installed.length > 1 ? t("settings.active_pack_hint") : undefined}
          value={active.id}
          options={installed.map((pack) => ({ value: pack.id, label: label(pack) }))}
          onChange={choose}
        />
      ) : (
        <p className="text-sm">{t("models.no_embedding_long")}</p>
      )}
      {(installed.length === 0 || more) && <div>{download}</div>}
      {save.isError && <ErrorMessage error={save.error} />}
    </section>
  );
}

/**
 * How a vector pack is named in model pickers: who embeds the queries and
 * the model, e.g. "Voyage AI · voyage-4-large"; the pack title otherwise.
 */
function embeddingName(pack: PackView, onDevice: string, language: string | undefined) {
  if (!pack.embedding) return packTitle(pack.title, pack.title_fallback, language);
  const where = pack.embedding.provider ? PROVIDER_NAMES[pack.embedding.provider] : onDevice;
  return `${where} · ${pack.embedding.model}`;
}
