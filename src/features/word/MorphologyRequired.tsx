import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { formatBytes, packTitle } from "@/features/packs/packFormat";
import { useCatalog } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

/**
 * What a feature that needs the morphology pack shows without it (spec §7.1):
 * the feature stays visible and says what to install. The Packs dialog then
 * shows the pack's terms, which must be read before it is downloaded.
 */
export function MorphologyRequired() {
  const { t, i18n } = useTranslation();
  const setPacksDialogOpen = useUiStore((state) => state.setPacksDialogOpen);
  const catalog = useCatalog();
  const language = i18n.resolvedLanguage;
  const pack = catalog.data?.packs.find((item) => item.kind === "morphology");
  return (
    <div className="grid justify-items-start gap-2 rounded-lg border p-3">
      <p className="text-sm">
        {pack
          ? t("word.pack_required_named", {
              title: packTitle(pack.title, pack.title_fallback, language),
              size: formatBytes(pack.size, language),
            })
          : t("word.pack_required")}
      </p>
      <Button
        variant="outline"
        size="sm"
        onClick={() => {
          setPacksDialogOpen(true, "morphology");
        }}
      >
        {t("word.open_packs")}
      </Button>
    </div>
  );
}
