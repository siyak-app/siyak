import { CheckIcon, ExternalLinkIcon, TriangleAlertIcon } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import type { Catalog, JobView, LocalizedList, PackView } from "@/lib/bindings";
import {
  appErrorCode,
  useAcknowledgeTerms,
  useCancelInstall,
  useInstallPack,
  useOpenPackSource,
  useUninstallPack,
} from "@/lib/queries";

import {
  formatBytes,
  jobFraction,
  languageNames,
  orphanedBy,
  packTitle,
  requiredPacks,
  requiredSize,
} from "./packFormat";
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

/** The terms in the interface language. */
function termsOf(terms: LocalizedList, language: string | undefined): string[] {
  return language === "tr" ? terms.tr : terms.en;
}

/** The terms of a pack, shown before download (spec §6.5). */
export function TermsList({ terms }: { terms: LocalizedList }) {
  const { t, i18n } = useTranslation();
  const items = termsOf(terms, i18n.resolvedLanguage);
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

/** Whether `pack` can be downloaded now (it may already be installed). */
function canInstall(pack: PackView): boolean {
  return pack.in_manifest && pack.supported && !pack.incompatible && !pack.bundled && !pack.job;
}

/** A one-line notice, e.g. why a pack cannot be installed. */
function Notice({ children }: { children: string }) {
  return (
    <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
      <TriangleAlertIcon className="size-3.5 shrink-0" aria-hidden />
      {children}
    </p>
  );
}

/**
 * One pack on the Packs screen: title, a short description, the download size
 * and one action; license, attribution, version and removal are under
 * Details. A pack with terms shows them, and asks for them to be accepted,
 * before it is downloaded (spec §6.5). The packs installed with it (a vector
 * pack's model) are part of the row: their size, updates and progress.
 */
export function PackRow({
  pack,
  catalog,
  onRemoved,
}: {
  pack: PackView;
  catalog: Catalog;
  onRemoved: () => void;
}) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const install = useInstallPack();
  const acknowledge = useAcknowledgeTerms();
  const uninstall = useUninstallPack();
  const openSource = useOpenPackSource();
  const failures = usePackFailures((state) => state.failures);
  const [confirming, setConfirming] = useState(false);
  const [accepted, setAccepted] = useState(false);

  const title = packTitle(pack.title, pack.title_fallback, language);
  const description =
    pack.description && (language === "tr" ? pack.description.tr : pack.description.en);
  const required = requiredPacks(pack, catalog);
  const extraSize = requiredSize(pack, catalog);
  // The pack itself, or for an installed pack what it needs and lacks.
  const target =
    canInstall(pack) && (!pack.installed || pack.update_available)
      ? pack
      : pack.installed
        ? required.find((p) => canInstall(p) && (!p.installed || p.update_available))
        : undefined;
  const needsTerms =
    target !== undefined && target.terms_ack_required && !target.terms_acknowledged;
  // Downloaded with the target: what it needs and lacks.
  const companions = target === pack ? required.filter((p) => !p.installed) : [];
  const hasTerms = [target, ...companions].some(
    (p) => p !== undefined && !p.installed && termsOf(p.license_terms, language).length > 0,
  );
  // Terms are shown before a first download and confirmed where they must be.
  const asksFirst = needsTerms || hasTerms;
  const replaced = pack.replaces && catalog.packs.find((p) => p.id === pack.replaces);
  const jobOwner = pack.job ? pack : required.find((p) => p.job);
  const failure = [pack, ...required].map((p) => failures[p.id]).find(Boolean);
  const mutationError = install.error ?? acknowledge.error ?? uninstall.error;
  const hasError = failure !== undefined || mutationError !== null;
  const errorCode = failure ?? appErrorCode(mutationError);

  const startInstall = () => {
    if (!target) return;
    const run = () => {
      setConfirming(false);
      install.mutate({ id: target.id, confirmReplace: target.replaces !== null });
    };
    if (needsTerms) {
      acknowledge.mutate({ id: target.id, version: target.version }, { onSuccess: run });
    } else {
      run();
    }
  };

  const remove = async () => {
    await uninstall.mutateAsync(pack.id);
    for (const orphan of orphanedBy(pack, catalog)) await uninstall.mutateAsync(orphan.id);
    onRemoved();
  };

  let action = t("packs.install");
  if (target?.installed && target.update_available)
    action = t("packs.update", { version: target.version });
  else if (target?.replaces) action = t("packs.replace");

  let hint: string | null = null;
  if (pack.embedding)
    hint = pack.embedding.provider
      ? t("packs.requires_key", { provider: PROVIDER_NAMES[pack.embedding.provider] })
      : t("packs.on_device");
  const size =
    target === pack
      ? [
          formatBytes(pack.size, language),
          extraSize > 0
            ? t("packs.model_included", { size: formatBytes(extraSize, language) })
            : null,
        ]
          .filter(Boolean)
          .join(" · ")
      : target && formatBytes(target.size, language);

  let status: string | null = null;
  if (pack.bundled) status = t("packs.builtin_badge");
  else if (pack.installed) status = t("packs.installed_badge");

  return (
    <li className="grid gap-2 rounded-lg border p-3">
      <div className="flex items-start gap-3">
        <div className="grid flex-1 gap-0.5">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium" lang={language}>
              {title}
            </span>
            {pack.style && (
              <Badge>{t(`packs.style_${pack.style}` as "packs.style_murattal")}</Badge>
            )}
          </div>
          {description && <p className="text-sm text-muted-foreground">{description}</p>}
          {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
          {size && !jobOwner && <p className="text-xs text-muted-foreground">{size}</p>}
        </div>
        {!jobOwner && target && !confirming && (
          <Button
            size="sm"
            disabled={install.isPending || acknowledge.isPending}
            onClick={() => {
              if (asksFirst) setConfirming(true);
              else startInstall();
            }}
          >
            {action}
          </Button>
        )}
        {!target && status && (
          <span className="flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
            <CheckIcon className="size-3.5" aria-hidden />
            {status}
          </span>
        )}
      </div>
      {!pack.supported && <Notice>{t("packs.unsupported")}</Notice>}
      {pack.supported && pack.incompatible && (
        <Notice>{t("packs.incompatible", { version: pack.min_app_version })}</Notice>
      )}
      {target === pack && replaced && (
        <Notice>
          {t("packs.replaces", {
            title: packTitle(replaced.title, replaced.title_fallback, language),
          })}
        </Notice>
      )}
      {confirming && target && (
        <div className="grid gap-2 rounded-md bg-muted/50 p-3">
          <TermsList terms={target.license_terms} />
          <p className="text-xs text-muted-foreground">
            {t("packs.license", { license: target.license })}
          </p>
          {target.attribution && <p className="text-xs">{target.attribution}</p>}
          {companions.map((p) => (
            <div key={p.id} className="grid gap-1">
              <p className="text-xs text-muted-foreground">
                {t("packs.comes_with", {
                  title: packTitle(p.title, p.title_fallback, language),
                  size: formatBytes(p.size, language),
                  license: p.license,
                })}
              </p>
              <TermsList terms={p.license_terms} />
            </div>
          ))}
          {needsTerms && <TermsCheckbox checked={accepted} onChange={setAccepted} />}
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              disabled={(needsTerms && !accepted) || install.isPending || acknowledge.isPending}
              onClick={startInstall}
            >
              {needsTerms ? t("packs.accept_and_install") : action}
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                setConfirming(false);
              }}
            >
              {t("packs.back")}
            </Button>
          </div>
        </div>
      )}
      {hasError && (
        <p role="alert" className="text-xs text-destructive">
          {errorCode ? t(`errors.${errorCode}`) : t("errors.unknown")}
        </p>
      )}
      {jobOwner?.job && <JobProgress id={jobOwner.id} job={jobOwner.job} />}
      <details className="text-xs">
        <summary className="cursor-pointer text-muted-foreground">{t("packs.details")}</summary>
        <div className="grid gap-1.5 pt-2">
          <p className="text-muted-foreground">
            {[
              pack.author,
              pack.lang && languageNames(language)(pack.lang),
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
          {required.map((p) => (
            <p key={p.id} className="text-muted-foreground">
              {t("packs.comes_with", {
                title: packTitle(p.title, p.title_fallback, language),
                size: formatBytes(p.size, language),
                license: p.license,
              })}
            </p>
          ))}
          {pack.attribution && <p>{pack.attribution}</p>}
          {!confirming && <TermsList terms={pack.license_terms} />}
          <div className="flex flex-wrap gap-2">
            {pack.source_url && (
              <Button
                variant="ghost"
                size="sm"
                className="px-0"
                onClick={() => {
                  openSource.mutate(pack.id);
                }}
              >
                <ExternalLinkIcon data-icon="inline-start" />
                {t("packs.source_link")}
              </Button>
            )}
            {pack.installed && !pack.bundled && !jobOwner && (
              <Button
                variant="outline"
                size="sm"
                disabled={uninstall.isPending}
                onClick={() => {
                  // A failure is shown through the mutation's error state.
                  remove().catch(() => undefined);
                }}
              >
                {t("packs.remove")}
              </Button>
            )}
          </div>
        </div>
      </details>
    </li>
  );
}
