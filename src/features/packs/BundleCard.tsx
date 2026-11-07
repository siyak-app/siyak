import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { BundleView, Catalog, PackView } from "@/lib/bindings";
import { appErrorCode, useAcknowledgeTerms, useInstallBundle } from "@/lib/queries";

import { formatBytes, packTitle, requiredPacks } from "./packFormat";
import { JobProgress, TermsCheckbox, TermsList } from "./PackRow";
import { usePackFailures } from "./usePackEvents";

/**
 * A pack set installed with one click (spec §6.3): its members, total size
 * and the terms of what it would download (spec §6.5), then each member's
 * progress. A model pack is listed as part of the vector pack that needs it.
 * Download first shows the license, attribution and terms of every pack it
 * would download, and asks for terms to be accepted where they must be.
 */
export function BundleCard({ bundle, catalog }: { bundle: BundleView; catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const install = useInstallBundle();
  const acknowledge = useAcknowledgeTerms();
  const failures = usePackFailures((state) => state.failures);
  const [confirming, setConfirming] = useState(false);
  const [accepted, setAccepted] = useState(false);

  const members = bundle.packs.flatMap((id) => catalog.packs.filter((p) => p.id === id));
  const missing = members.filter((p) => bundle.missing.includes(p.id));
  const unacknowledged = missing.filter((p) => p.terms_ack_required && !p.terms_acknowledged);
  const jobs = members.filter((p) => p.job);
  const replaces = missing.some((p) => p.replaces !== null);
  const error = install.error ?? acknowledge.error;
  const errorCode = appErrorCode(error);
  const text = (value: { tr: string; en: string }) => (language === "tr" ? value.tr : value.en);
  const memberIds = new Set(bundle.packs);
  const needed = new Set(members.flatMap((p) => p.requires));
  // A required model is shown with the pack that needs it.
  const listed = members.filter((p) => !(p.kind === "model" && needed.has(p.id)));
  const missingSize = (pack: PackView) =>
    [pack, ...requiredPacks(pack, catalog).filter((p) => memberIds.has(p.id))]
      .filter((p) => bundle.missing.includes(p.id))
      .reduce((sum, p) => sum + (p.size ?? 0), 0);

  const start = async () => {
    setConfirming(false);
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
        {listed.map((pack) => {
          const size = missingSize(pack);
          return (
            <li key={pack.id} className="flex flex-wrap items-center gap-x-2">
              <span>{packTitle(pack.title, pack.title_fallback, language)}</span>
              <span className="text-xs text-muted-foreground">
                {size === 0 ? t("packs.installed_badge") : formatBytes(size, language)}
              </span>
            </li>
          );
        })}
      </ul>
      {confirming && jobs.length === 0 && (
        <div className="grid gap-3 rounded-md bg-muted/50 p-3">
          <ul className="grid gap-3">
            {missing.map((pack) => (
              <li key={pack.id} className="grid gap-1 text-xs">
                <span className="font-medium">
                  {packTitle(pack.title, pack.title_fallback, language)}
                </span>
                <span className="text-muted-foreground">
                  {t("packs.license", { license: pack.license })}
                </span>
                {pack.attribution && <span>{pack.attribution}</span>}
                <TermsList terms={pack.license_terms} />
              </li>
            ))}
          </ul>
          {unacknowledged.length > 0 && <TermsCheckbox checked={accepted} onChange={setAccepted} />}
        </div>
      )}
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
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={
                (confirming && unacknowledged.length > 0 && !accepted) ||
                install.isPending ||
                acknowledge.isPending
              }
              onClick={() => {
                if (!confirming) {
                  setConfirming(true);
                  return;
                }
                // A failure is shown through the mutation's error state.
                start().catch(() => undefined);
              }}
            >
              {confirming && unacknowledged.length > 0
                ? t("packs.accept_and_install")
                : t("packs.install_bundle", { size: formatBytes(bundle.size, language) })}
            </Button>
            {confirming && (
              <Button
                variant="ghost"
                onClick={() => {
                  setConfirming(false);
                }}
              >
                {t("packs.back")}
              </Button>
            )}
          </div>
        )
      )}
    </section>
  );
}
