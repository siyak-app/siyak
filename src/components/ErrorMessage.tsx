import { useTranslation } from "react-i18next";

import { appErrorCode } from "@/lib/queries";

/** Shows a translated message for an error thrown by a query or mutation. */
export function ErrorMessage({ error }: { error: unknown }) {
  const { t } = useTranslation();
  const code = appErrorCode(error);
  return (
    <div role="alert" className="m-6 rounded-lg border border-destructive/30 p-4">
      <p className="font-medium text-destructive">{t("errors.title")}</p>
      <p className="mt-1 text-muted-foreground">
        {code ? t(`errors.${code}`) : t("errors.unknown")}
      </p>
    </div>
  );
}
