import { XIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useCoreStatus } from "@/lib/queries";

/**
 * Debug builds only: which dataset is loaded (`SIYAK_DEV_DATA`), so it is
 * always clear whether the reader shows the small development data or the
 * full `core.db`.
 */
export function DevDataBadge() {
  const { t } = useTranslation();
  const status = useCoreStatus();
  if (!import.meta.env.DEV || !status.data?.seed) return null;
  return (
    <span className="rounded-md border border-dashed px-2 py-0.5 text-xs text-muted-foreground">
      {t("layout.dev_badge", { seed: status.data.seed, version: status.data.core_version })}
    </span>
  );
}

/**
 * Reports a failed core refresh (spec §6.2). The app keeps working on the
 * previous data, so the message can be dismissed.
 */
export function CoreRefreshBanner() {
  const { t } = useTranslation();
  const status = useCoreStatus();
  const [dismissed, setDismissed] = useState(false);
  const error = status.data?.refresh_error;
  if (!error || dismissed) return null;
  return (
    <div
      role="alert"
      className="flex items-start gap-3 border-b border-destructive/30 bg-destructive/5 px-4 py-2 text-sm"
    >
      <p className="flex-1">{t(`errors.${error.code}`)}</p>
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={t("common.dismiss")}
        onClick={() => {
          setDismissed(true);
        }}
      >
        <XIcon />
      </Button>
    </div>
  );
}
