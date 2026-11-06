import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { formatBytes } from "@/features/packs/packFormat";
import { useNetworkLog } from "@/lib/queries";

const PURPOSES = ["manifest", "pack", "embed", "update", "chat", "translate"] as const;

/** Settings → Privacy: the connection log (spec §14.1). */
export function PrivacySettings() {
  const { t, i18n } = useTranslation();
  const log = useNetworkLog();
  const language = i18n.resolvedLanguage;
  const time = new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "medium" });
  const purpose = (value: string) =>
    (PURPOSES as readonly string[]).includes(value)
      ? t(`settings.network_purpose_${value as (typeof PURPOSES)[number]}`)
      : value;

  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-medium">{t("settings.network_log")}</h3>
      <p className="text-xs text-muted-foreground">{t("settings.network_log_description")}</p>
      {log.isError && <ErrorMessage error={log.error} />}
      {log.data?.length === 0 && (
        <p className="text-sm text-muted-foreground">{t("settings.network_log_empty")}</p>
      )}
      {log.data && log.data.length > 0 && (
        <table className="w-full text-xs">
          <thead className="text-start text-muted-foreground">
            <tr>
              <th className="py-1 text-start font-normal">{t("settings.network_time")}</th>
              <th className="py-1 text-start font-normal">{t("settings.network_host")}</th>
              <th className="py-1 text-start font-normal">{t("settings.network_purpose")}</th>
              <th className="py-1 text-end font-normal">{t("settings.network_result")}</th>
            </tr>
          </thead>
          <tbody>
            {log.data.map((entry, index) => (
              // Entries have no id and may repeat; the order is stable.
              <tr key={index} className="border-t">
                <td className="py-1 pe-2 tabular-nums">{time.format(new Date(entry.at))}</td>
                <td className="py-1 pe-2 font-mono">{entry.host}</td>
                <td className="py-1 pe-2">{purpose(entry.purpose)}</td>
                <td className="py-1 text-end tabular-nums">
                  {entry.status === null
                    ? t("settings.network_no_response")
                    : `${String(entry.status)} · ${formatBytes(entry.bytes, language)}`}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
