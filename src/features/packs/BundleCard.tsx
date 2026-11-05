import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { BundleView, Catalog } from "@/lib/bindings";
import { appErrorCode, useAcknowledgeTerms, useInstallBundle } from "@/lib/queries";

import { formatBytes, packTitle } from "./packFormat";
import { JobProgress, TermsCheckbox, TermsList } from "./PackRow";
import { usePackFailures } from "./usePackEvents";

/**
 * A pack set installed with one click (spec §6.3): its members, total size
 * and the license and terms of what it would download (spec §6.5), then each
 * member's progress.
 */
export function BundleCard({ bundle, catalog }: { bundle: BundleView; catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const install = useInstallBundle();
  const acknowledge = useAcknowledgeTerms();
  const failures = usePackFailures((state) => state.failures);
  const [accepted, setAccepted] = useState(false);

  const members = bundle.packs.flatMap((id) => catalog.packs.filter((p) => p.id === id));
  const missing = members.filter((p) => bundle.missing.includes(p.id));
  const unacknowledged = missing.filter((p) => p.terms_ack_required && !p.terms_acknowledged);
  const jobs = members.filter((p) => p.job);
  const replaces = missing.some((p) => p.replaces !== null);
  const error = install.error ?? acknowledge.error;
  const errorCode = appErrorCode(error);
  const text = (value: { tr: string; en: string }) => (language === "tr" ? value.tr : value.en);

  const start = async () => {
    for (const pack of unacknowledged) {
      await acknowledge.mutateAsync({ id: pack.id, version: pack.version });
    }
    install.mutate({ id: bundle.id, confirmReplace: replaces });
  };

  return (
    <section className="grid gap-3 rounded-lg border p-4">
      <div className="grid gap-1">
        <h3 className="font-medium">{text(bundle.title)}</h3>
        {bundle.description && (
          <p className="text-sm text-muted-foreground">{text(bundle.description)}</p>
        )}
      </div>
      <ul className="grid gap-1 text-sm">
        {members.map((pack) => (
          <li key={pack.id} className="flex flex-wrap items-center gap-x-2">
            <span>{packTitle(pack.title, pack.title_fallback, language)}</span>
            <span className="text-xs text-muted-foreground">
              {pack.installed && !bundle.missing.includes(pack.id)
                ? t("packs.installed_badge")
                : formatBytes(pack.size, language)}
            </span>
          </li>
        ))}
      </ul>
      {jobs.length === 0 &&
        missing.map((pack) => (
          <div key={pack.id} className="grid gap-1 text-xs">
            <span className="font-medium">
              {packTitle(pack.title, pack.title_fallback, language)}
            </span>
            <span className="text-muted-foreground">
              {t("packs.license", { license: pack.license })}
            </span>
            {pack.attribution && <span>{pack.attribution}</span>}
            <TermsList terms={pack.license_terms} />
          </div>
        ))}
      {jobs.map((pack) =>
        pack.job ? (
          <div key={pack.id} className="grid gap-1">
            <span className="text-xs">{packTitle(pack.title, pack.title_fallback, language)}</span>
            <JobProgress id={pack.id} job={pack.job} />
          </div>
        ) : null,
      )}
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {errorCode ? t(`errors.${errorCode}`) : t("errors.unknown")}
        </p>
      )}
      {members.map((pack) => {
        const failure = failures[pack.id];
        return (
          failure && (
            <p key={pack.id} role="alert" className="text-xs text-destructive">
              {packTitle(pack.title, pack.title_fallback, language)}: {t(`errors.${failure}`)}
            </p>
          )
        );
      })}
      {bundle.missing.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("packs.bundle_installed")}</p>
      ) : (
        jobs.length === 0 && (
          <div className="grid gap-2">
            {unacknowledged.length > 0 && (
              <TermsCheckbox checked={accepted} onChange={setAccepted} />
            )}
            <Button
              className="justify-self-start"
              disabled={
                (unacknowledged.length > 0 && !accepted) ||
                install.isPending ||
                acknowledge.isPending
              }
              onClick={() => {
                // A failure is shown through the mutation's error state.
                start().catch(() => undefined);
              }}
            >
              {t("packs.install_bundle", { size: formatBytes(bundle.size, language) })}
            </Button>
          </div>
        )
      )}
    </section>
  );
}
