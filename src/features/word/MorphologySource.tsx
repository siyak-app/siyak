import { useTranslation } from "react-i18next";

import { useMorphologyPack, useOpenPackSource } from "@/lib/queries";

/** The source and its link, required wherever the data is shown (spec §5.3.1). */
export function MorphologySource() {
  const { t } = useTranslation();
  const pack = useMorphologyPack();
  const openSource = useOpenPackSource();
  if (!pack.data) return null;
  const { id, source } = pack.data;
  return (
    <p className="text-xs text-muted-foreground">
      {t("word.source")}{" "}
      <button
        type="button"
        className="underline underline-offset-2 hover:text-foreground"
        onClick={() => {
          openSource.mutate(id);
        }}
      >
        {source}
      </button>
    </p>
  );
}
