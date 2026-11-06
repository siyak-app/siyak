import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { formatBytes, packTitle } from "@/features/packs/packFormat";
import type { Catalog, SemanticStatus } from "@/lib/bindings";
import { useCatalog } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { PROVIDER_NAMES } from "./providerNames";

/**
 * What a feature that needs semantic search shows while it cannot run (spec
 * §7.1, §7.4): the feature stays visible and says what is missing. Missing
 * packs lead to the Packs dialog, which shows their terms before download.
 */
export function SemanticRequired({
  status,
}: {
  status: Exclude<SemanticStatus, { state: "ready" }>;
}) {
  const { t, i18n } = useTranslation();
  const setPacksDialogOpen = useUiStore((state) => state.setPacksDialogOpen);
  const needsPacks = status.state === "no_vector_pack" || status.state === "model_pack_missing";
  const catalog = useCatalog(needsPacks);
  const language = i18n.resolvedLanguage;

  let message: string;
  switch (status.state) {
    case "no_vector_pack": {
      const offer = catalog.data && vectorOffer(catalog.data);
      message = offer
        ? t("semantic.pack_required_named", {
            title: packTitle(offer.pack.title, offer.pack.title_fallback, language),
            size: formatBytes(offer.size, language),
          })
        : t("semantic.pack_required");
      break;
    }
    case "model_pack_missing":
      message = t("semantic.model_missing", { pack: status.model_pack });
      break;
    case "key_missing":
      message = t("semantic.key_missing", { provider: PROVIDER_NAMES[status.provider] });
      break;
    case "base_url_missing":
      message = t("semantic.base_url_missing");
      break;
    default:
      message = t("semantic.unusable", { pack: status.pack });
  }
  return (
    <div className="grid justify-items-start gap-2 rounded-lg border p-3">
      <p className="text-sm">{message}</p>
      {needsPacks && (
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setPacksDialogOpen(true);
          }}
        >
          {t("semantic.open_packs")}
        </Button>
      )}
    </div>
  );
}

/**
 * The vector pack to suggest: one from a bundle (the recommended one comes
 * first), else the first listed; its size includes the packs it requires.
 */
function vectorOffer(catalog: Catalog) {
  const vectors = catalog.packs.filter((pack) => pack.kind === "vectors" && pack.supported);
  const bundled = new Set(catalog.bundles.flatMap((bundle) => bundle.packs));
  const pack = vectors.find((item) => bundled.has(item.id)) ?? vectors[0];
  if (!pack) return null;
  const required = catalog.packs.filter((item) => pack.requires.includes(item.id));
  let size: number | null = 0;
  for (const item of [pack, ...required]) {
    size = size === null || item.size === null ? null : size + item.size;
  }
  return { pack, size };
}
