import { PackageIcon, RefreshCwIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { Catalog, PackView } from "@/lib/bindings";
import {
  appErrorCode,
  useCatalog,
  useOpenPackSource,
  useRefreshCatalog,
  useVacuumContent,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { BundleCard } from "./BundleCard";
import { formatBytes, KIND_ORDER, packTitle } from "./packFormat";
import { Badge, PackRow } from "./PackRow";

/** The Packs screen (spec §13.3 item 8) with About → Sources, as a dialog. */
export function PacksDialog() {
  const { t } = useTranslation();
  const open = useUiStore((state) => state.packsDialogOpen);
  const setOpen = useUiStore((state) => state.setPacksDialogOpen);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon-sm" aria-label={t("layout.open_packs")}>
          <PackageIcon />
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[85vh] flex-col sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("packs.title")}</DialogTitle>
          <DialogDescription>{t("packs.description")}</DialogDescription>
        </DialogHeader>
        {open && <PacksContent />}
      </DialogContent>
    </Dialog>
  );
}

function PacksContent() {
  const { t } = useTranslation();
  const catalog = useCatalog();
  if (catalog.isPending) {
    return <p className="text-sm text-muted-foreground">{t("packs.loading")}</p>;
  }
  if (catalog.isError) return <ErrorMessage error={catalog.error} />;
  return (
    <Tabs defaultValue="packs" className="min-h-0 flex-1">
      <TabsList className="w-full">
        <TabsTrigger value="packs">{t("packs.tab_packs")}</TabsTrigger>
        <TabsTrigger value="sources">{t("packs.tab_sources")}</TabsTrigger>
      </TabsList>
      <div className="-mx-1 min-h-0 overflow-y-auto px-1 pt-3">
        <TabsContent value="packs">
          <PacksTab catalog={catalog.data} />
        </TabsContent>
        <TabsContent value="sources">
          <SourcesTab catalog={catalog.data} />
        </TabsContent>
      </div>
    </Tabs>
  );
}

/** Where the pack list came from and when; refreshes it on request. */
function CatalogStatus({ catalog }: { catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const refresh = useRefreshCatalog();
  const fetched =
    catalog.fetched_at === null
      ? null
      : new Intl.DateTimeFormat(i18n.resolvedLanguage, {
          dateStyle: "medium",
          timeStyle: "short",
        }).format(new Date(catalog.fetched_at));
  const code = catalog.error?.code;
  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        {fetched && <span>{t("packs.fetched_at", { time: fetched })}</span>}
        {catalog.source === "local" && <Badge>{t("packs.source_local")}</Badge>}
        <Button
          variant="ghost"
          size="sm"
          className="ms-auto"
          disabled={refresh.isPending}
          onClick={() => {
            refresh.mutate();
          }}
        >
          <RefreshCwIcon data-icon="inline-start" />
          {t("packs.refresh")}
        </Button>
      </div>
      {code && (
        <p role="status" className="rounded-lg border p-3 text-sm">
          {catalog.offline && catalog.packs.some((p) => p.in_manifest)
            ? t("packs.offline")
            : t(`errors.${code}`)}
        </p>
      )}
      {refresh.isError && (
        <p role="alert" className="text-xs text-destructive">
          {t(`errors.${appErrorCode(refresh.error) ?? "unknown"}`)}
        </p>
      )}
    </div>
  );
}

function PacksTab({ catalog }: { catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const vacuum = useVacuumContent();
  const [removed, setRemoved] = useState(false);

  const groups = new Map<string, PackView[]>();
  for (const kind of KIND_ORDER) groups.set(kind, []);
  for (const pack of catalog.packs) {
    const kind = (KIND_ORDER as readonly string[]).includes(pack.kind) ? pack.kind : "other";
    groups.set(kind, [...(groups.get(kind) ?? []), pack]);
  }
  const titleOf = (id: string | null) => {
    const pack = id === null ? undefined : catalog.packs.find((p) => p.id === id);
    return pack ? packTitle(pack.title, pack.title_fallback, language) : id;
  };

  return (
    <div className="grid gap-5">
      <CatalogStatus catalog={catalog} />
      {removed && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border p-3 text-sm">
          <span className="flex-1">
            {vacuum.data
              ? t("packs.vacuum_done", {
                  size: formatBytes(
                    Math.max(0, (vacuum.data.before ?? 0) - (vacuum.data.after ?? 0)),
                    language,
                  ),
                })
              : t("packs.removed_vacuum")}
          </span>
          {!vacuum.data && (
            <Button
              size="sm"
              variant="outline"
              disabled={vacuum.isPending}
              onClick={() => {
                vacuum.mutate();
              }}
            >
              {t("packs.vacuum")}
            </Button>
          )}
        </div>
      )}
      {catalog.bundles.length > 0 && (
        <section className="grid gap-2">
          <h3 className="text-sm font-medium">{t("packs.bundles")}</h3>
          {catalog.bundles.map((bundle) => (
            <BundleCard key={bundle.id} bundle={bundle} catalog={catalog} />
          ))}
        </section>
      )}
      <section className="grid gap-4">
        <h3 className="text-sm font-medium">{t("packs.individual")}</h3>
        {[...groups].map(([kind, packs]) =>
          packs.length === 0 ? null : (
            <div key={kind} className="grid gap-2">
              <h4 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                {t(`packs.kind_${kind as (typeof KIND_ORDER)[number] | "other"}`)}
              </h4>
              <ul className="grid gap-2">
                {packs.map((pack) => (
                  <PackRow
                    key={pack.id}
                    pack={pack}
                    replacedTitle={titleOf(pack.replaces)}
                    onRemoved={() => {
                      vacuum.reset();
                      setRemoved(true);
                    }}
                  />
                ))}
              </ul>
            </div>
          ),
        )}
      </section>
    </div>
  );
}

/** About → Sources (spec §6.5): every installed pack with its license and attribution. */
function SourcesTab({ catalog }: { catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const openSource = useOpenPackSource();
  const installed = catalog.packs.filter((p) => p.installed);
  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">{t("packs.sources_description")}</p>
      <ul className="grid gap-2">
        {installed.map((pack) => (
          <li key={pack.id} className="grid gap-1 rounded-lg border p-3 text-sm">
            <span className="font-medium">
              {packTitle(pack.title, pack.title_fallback, i18n.resolvedLanguage)}
            </span>
            <span className="text-xs text-muted-foreground">
              {[pack.author, t("packs.license", { license: pack.license })]
                .filter(Boolean)
                .join(" · ")}
            </span>
            {pack.attribution && <span className="text-xs">{pack.attribution}</span>}
            {pack.source_url && (
              <Button
                variant="link"
                size="sm"
                className="justify-self-start px-0"
                onClick={() => {
                  openSource.mutate(pack.id);
                }}
              >
                {pack.source_url}
              </Button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
