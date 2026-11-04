import { useTranslation } from "react-i18next";

import { ConfirmButton } from "@/components/ConfirmButton";
import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { useExportUserData, useImportUserData } from "@/lib/queries";

/** Backing up and restoring the user's data as a JSON file (spec §11.2). */
export function BackupSettings() {
  const { t } = useTranslation();
  const backup = useExportUserData();
  const restore = useImportUserData();
  const report = restore.data;
  return (
    <div className="grid gap-6 text-sm">
      <section className="grid justify-items-start gap-2">
        <h3 className="font-medium">{t("settings.backup_title")}</h3>
        <p className="text-muted-foreground">{t("settings.backup_hint")}</p>
        <Button
          variant="outline"
          size="sm"
          disabled={backup.isPending}
          onClick={() => {
            backup.mutate();
          }}
        >
          {t("settings.backup_export")}
        </Button>
        {backup.data && (
          <p role="status" className="wrap-anywhere text-muted-foreground">
            {t("settings.backup_saved", { path: backup.data })}
          </p>
        )}
        {backup.isError && <ErrorMessage error={backup.error} />}
      </section>
      <section className="grid justify-items-start gap-2">
        <h3 className="font-medium">{t("settings.import_title")}</h3>
        <p className="text-muted-foreground">{t("settings.import_merge_hint")}</p>
        <Button
          variant="outline"
          size="sm"
          disabled={restore.isPending}
          onClick={() => {
            restore.mutate("merge");
          }}
        >
          {t("settings.import_merge")}
        </Button>
        <p className="text-muted-foreground">{t("settings.import_replace_hint")}</p>
        <ConfirmButton
          size="sm"
          label={t("settings.import_replace")}
          confirmLabel={t("settings.import_replace_confirm")}
          onConfirm={() => {
            restore.mutate("replace");
          }}
        />
        {report && (
          <div role="status" className="grid gap-1">
            <p>
              {t("settings.import_report", {
                bookmarks: report.bookmarks,
                notes: report.notes,
                collections: report.collections,
                items: report.items,
                translations: report.tafsir_translations,
              })}
            </p>
            {report.skipped > 0 && (
              <p className="text-muted-foreground">
                {t("settings.import_skipped", { count: report.skipped })}
              </p>
            )}
          </div>
        )}
        {restore.isError && <ErrorMessage error={restore.error} />}
      </section>
    </div>
  );
}
