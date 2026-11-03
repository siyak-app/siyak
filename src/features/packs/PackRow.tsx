import { ExternalLinkIcon } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type { JobView, LocalizedList, PackView } from "@/lib/bindings";
import {
  appErrorCode,
  useAcknowledgeTerms,
  useCancelInstall,
  useInstallPack,
  useOpenPackSource,
  useUninstallPack,
} from "@/lib/queries";

import { formatBytes, jobFraction, packTitle } from "./packFormat";
import { usePackFailures } from "./usePackEvents";

/** A small label next to a pack title. */
export function Badge({ children }: { children: string }) {
  return (
    <span className="rounded-full border px-2 py-0.5 text-xs font-normal text-muted-foreground">
      {children}
    </span>
  );
}

/** Progress of a queued or running job, with a cancel button. */
export function JobProgress({ id, job }: { id: string; job: JobView }) {
  const { t, i18n } = useTranslation();
  const cancel = useCancelInstall();
  const fraction = jobFraction(job);
  const language = i18n.resolvedLanguage;
  let label = t("packs.queued");
  if (job.state === "running") {
    label =
      job.phase === "download"
        ? t("packs.phase_download", {
            done: formatBytes(job.done, language),
            total: formatBytes(job.total, language),
          })
        : t(`packs.phase_${job.phase}`);
  }
  return (
    <div className="flex items-center gap-2">
      <div className="grid flex-1 gap-1">
        <span className="text-xs text-muted-foreground">{label}</span>
        <div
          role="progressbar"
          aria-label={label}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={fraction === null ? undefined : Math.round(fraction * 100)}
          className="h-1.5 overflow-hidden rounded-full bg-muted"
        >
          <div
            className="h-full bg-primary transition-[width]"
            style={{ width: `${String(Math.round((fraction ?? 0) * 100))}%` }}
          />
        </div>
      </div>
      <Button
        variant="outline"
        size="sm"
        disabled={cancel.isPending}
        onClick={() => {
          cancel.mutate(id);
        }}
      >
        {t("packs.cancel")}
      </Button>
    </div>
  );
}

/** The terms of a pack, shown before download (spec §6.5). */
export function TermsList({ terms }: { terms: LocalizedList }) {
  const { t, i18n } = useTranslation();
  const items = i18n.resolvedLanguage === "tr" ? terms.tr : terms.en;
  if (items.length === 0) return null;
  return (
    <div className="grid gap-1 text-xs">
      <span className="font-medium">{t("packs.terms_title")}</span>
      <ul className="list-disc ps-5 text-muted-foreground">
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}

/** A checkbox that confirms the terms were read. */
export function TermsCheckbox({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <div className="flex items-center gap-2 text-sm">
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(value) => {
          onChange(value === true);
        }}
      />
      <label htmlFor={id}>{t("packs.terms_accept")}</label>
    </div>
  );
}

/** One pack on the Packs screen: details, terms, state and actions. */
export function PackRow({
  pack,
  replacedTitle,
  onRemoved,
}: {
  pack: PackView;
  /** Title of the morphology pack this one would replace. */
  replacedTitle: string | null;
  onRemoved: () => void;
}) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const install = useInstallPack();
  const acknowledge = useAcknowledgeTerms();
  const uninstall = useUninstallPack();
  const openSource = useOpenPackSource();
  const failure = usePackFailures((state) => state.failures[pack.id]);
  const [accepted, setAccepted] = useState(false);

  const title = packTitle(pack.title, pack.title_fallback, language);
  const description =
    pack.description && (language === "tr" ? pack.description.tr : pack.description.en);
  const needsTerms = pack.terms_ack_required && !pack.terms_acknowledged;
  const installable =
    pack.in_manifest && pack.supported && !pack.incompatible && !pack.bundled && !pack.job;
  const showInstall = installable && (!pack.installed || pack.update_available);
  const mutationError = install.error ?? acknowledge.error ?? uninstall.error;
  const hasError = failure !== undefined || mutationError !== null;
  const errorCode = failure ?? appErrorCode(mutationError);

  const startInstall = () => {
    const run = () => {
      install.mutate({ id: pack.id, confirmReplace: pack.replaces !== null });
    };
    if (needsTerms) {
      acknowledge.mutate({ id: pack.id, version: pack.version }, { onSuccess: run });
    } else {
      run();
    }
  };

  let action = t("packs.install");
  if (pack.installed && pack.update_available)
    action = t("packs.update", { version: pack.version });
  else if (pack.replaces) action = t("packs.replace");

  return (
    <li className="grid gap-2 rounded-lg border p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium" lang={language}>
          {title}
        </span>
        {pack.bundled && <Badge>{t("packs.builtin_badge")}</Badge>}
        {pack.installed && !pack.bundled && <Badge>{t("packs.installed_badge")}</Badge>}
        {pack.update_available && <Badge>{t("packs.update_badge")}</Badge>}
      </div>
      {description && <p className="text-sm text-muted-foreground">{description}</p>}
      <p className="text-xs text-muted-foreground">
        {[
          pack.author,
          pack.lang,
          t("packs.version", { version: pack.installed_version ?? pack.version }),
          pack.in_manifest
            ? t("packs.size", {
                size: formatBytes(pack.size, language),
                installed: formatBytes(pack.installed_size, language),
              })
            : null,
          t("packs.license", { license: pack.license }),
        ]
          .filter(Boolean)
          .join(" · ")}
      </p>
      {pack.attribution && <p className="text-xs">{pack.attribution}</p>}
      {!pack.installed && <TermsList terms={pack.license_terms} />}
      {!pack.supported && <p className="text-xs text-muted-foreground">{t("packs.unsupported")}</p>}
      {pack.supported && pack.incompatible && (
        <p className="text-xs text-muted-foreground">{t("packs.incompatible")}</p>
      )}
      {showInstall && replacedTitle && (
        <p className="text-xs text-muted-foreground">
          {t("packs.replaces", { title: replacedTitle })}
        </p>
      )}
      {showInstall && needsTerms && <TermsCheckbox checked={accepted} onChange={setAccepted} />}
      {hasError && (
        <p role="alert" className="text-xs text-destructive">
          {errorCode ? t(`errors.${errorCode}`) : t("errors.unknown")}
        </p>
      )}
      {pack.job ? (
        <JobProgress id={pack.id} job={pack.job} />
      ) : (
        <div className="flex flex-wrap gap-2">
          {showInstall && (
            <Button
              size="sm"
              disabled={(needsTerms && !accepted) || install.isPending || acknowledge.isPending}
              onClick={startInstall}
            >
              {action}
            </Button>
          )}
          {pack.installed && !pack.bundled && (
            <Button
              variant="outline"
              size="sm"
              disabled={uninstall.isPending}
              onClick={() => {
                uninstall.mutate(pack.id, { onSuccess: onRemoved });
              }}
            >
              {t("packs.remove")}
            </Button>
          )}
          {pack.source_url && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                openSource.mutate(pack.id);
              }}
            >
              <ExternalLinkIcon data-icon="inline-start" />
              {t("packs.source_link")}
            </Button>
          )}
        </div>
      )}
    </li>
  );
}
