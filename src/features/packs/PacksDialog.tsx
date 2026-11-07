import { PackageIcon, RefreshCwIcon } from "lucide-react";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { Catalog, PackView } from "@/lib/bindings";
import {
  appErrorCode,
  useCatalog,
  useOpenPackSource,
  useRefreshCatalog,
  useVacuumContent,
} from "@/lib/queries";
import { PACK_CATEGORIES, type PackCategory, useUiStore } from "@/stores/ui";

import { BundleCard } from "./BundleCard";
import {
  formatBytes,
  groupByLanguage,
  languageNames,
  matchesQuery,
  packCategory,
  packTitle,
} from "./packFormat";
import { Badge, PackRow } from "./PackRow";

/**
 * The Packs screen (spec §13.3 item 8) with About → Sources, as a dialog: a
 * menu of pack types on the side, the packs of the chosen type beside it.
 */
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
      <DialogContent className="flex h-[85vh] flex-col sm:max-w-4xl">
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
  return <PacksBrowser catalog={catalog.data} />;
}

/** Recommended sets until something is installed, then the translations. */
function defaultCategory(catalog: Catalog): PackCategory {
  const started = catalog.packs.some((p) => p.installed && !p.bundled);
  return catalog.bundles.length > 0 && !started ? "recommended" : "translation";
}

function PacksBrowser({ catalog }: { catalog: Catalog }) {
  const { t, i18n } = useTranslation();
  const requested = useUiStore((state) => state.packsCategory);
  const [category, setCategory] = useState<PackCategory>(
    () => requested ?? defaultCategory(catalog),
  );
  const vacuum = useVacuumContent();
  const [removed, setRemoved] = useState(false);

  const listed = new Map<PackCategory, PackView[]>(PACK_CATEGORIES.map((c) => [c, []]));
  for (const pack of catalog.packs) {
    const section = packCategory(pack, catalog);
    if (section) listed.get(section)?.push(pack);
  }
  const shown = (section: PackCategory) => {
    if (section === category || section === "sources" || section === "audio") return true;
    if (section === "recommended") return catalog.bundles.length > 0;
    return (listed.get(section)?.length ?? 0) > 0;
  };
  const onRemoved = () => {
    vacuum.reset();
    setRemoved(true);
  };

  return (
    <Tabs
      orientation="vertical"
      value={category}
      onValueChange={(value) => {
        setCategory(value as PackCategory);
      }}
      className="min-h-0 flex-1 gap-4"
    >
      <TabsList variant="line" className="w-56 shrink-0 self-start">
        {PACK_CATEGORIES.filter(shown).map((section) => {
          const installed = listed.get(section)?.filter((p) => p.installed).length ?? 0;
          return (
            <TabsTrigger
              key={section}
              value={section}
              className={section === "sources" ? "mt-2 border-t pt-2" : undefined}
            >
              <span className="flex-1 text-start">{t(`packs.category_${section}`)}</span>
              {section !== "sources" && installed > 0 && (
                <span className="text-xs text-muted-foreground tabular-nums">{installed}</span>
              )}
            </TabsTrigger>
          );
        })}
      </TabsList>
      <div className="-mx-1 grid min-h-0 flex-1 content-start gap-4 overflow-y-auto px-1">
        <CatalogStatus catalog={catalog} />
        {removed && (
          <div className="flex flex-wrap items-center gap-2 rounded-lg border p-3 text-sm">
            <span className="flex-1">
              {vacuum.data
                ? t("packs.vacuum_done", {
                    size: formatBytes(
                      Math.max(0, (vacuum.data.before ?? 0) - (vacuum.data.after ?? 0)),
                      i18n.resolvedLanguage,
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
        <TabsContent value="recommended" className="grid content-start gap-2">
          {catalog.bundles.map((bundle) => (
            <BundleCard key={bundle.id} bundle={bundle} catalog={catalog} />
          ))}
        </TabsContent>
        {PACK_CATEGORIES.filter((c) => c !== "recommended" && c !== "sources").map((section) => (
          <TabsContent key={section} value={section}>
            <PackSection
              section={section}
              packs={listed.get(section) ?? []}
              catalog={catalog}
              onRemoved={onRemoved}
            />
          </TabsContent>
        ))}
        <TabsContent value="sources">
          <SourcesTab catalog={catalog} />
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

/** Lists above this many packs can be searched and filtered. */
const SEARCHABLE_FROM = 4;

/**
 * The packs of one type, translations and tafsirs grouped by language, with a
 * search by title or author and an "installed only" filter for longer lists.
 */
function PackSection({
  section,
  packs,
  catalog,
  onRemoved,
}: {
  section: PackCategory;
  packs: PackView[];
  catalog: Catalog;
  onRemoved: () => void;
}) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const installedOnlyId = useId();
  const [query, setQuery] = useState("");
  const [installedOnly, setInstalledOnly] = useState(false);

  if (packs.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {section === "audio" ? t("packs.audio_empty") : t("packs.no_results")}
      </p>
    );
  }
  const visible = packs.filter(
    (pack) => (!installedOnly || pack.installed) && matchesQuery(pack, query),
  );
  const names = languageNames(language);
  const groups: [string | null, PackView[]][] =
    section === "translation" || section === "tafsir"
      ? groupByLanguage(visible, language)
      : [[null, visible]];
  const list = (items: PackView[]) => (
    <ul className="grid gap-2">
      {items.map((pack) => (
        <PackRow key={pack.id} pack={pack} catalog={catalog} onRemoved={onRemoved} />
      ))}
    </ul>
  );

  return (
    <div className="grid gap-4">
      {packs.length >= SEARCHABLE_FROM && (
        <div className="flex flex-wrap items-center gap-3">
          <Input
            type="search"
            className="max-w-xs flex-1"
            aria-label={t("packs.search_label")}
            placeholder={t("packs.search_label")}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
          />
          <div className="flex items-center gap-2 text-sm">
            <Checkbox
              id={installedOnlyId}
              checked={installedOnly}
              onCheckedChange={(value) => {
                setInstalledOnly(value === true);
              }}
            />
            <label htmlFor={installedOnlyId}>{t("packs.installed_only")}</label>
          </div>
        </div>
      )}
      {visible.length === 0 && (
        <p className="text-sm text-muted-foreground">{t("packs.no_results")}</p>
      )}
      {groups.length === 1
        ? list(groups[0]?.[1] ?? [])
        : groups.map(([lang, items]) => (
            <section key={lang ?? ""} className="grid gap-2">
              <h4 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                {names(lang) ?? t("packs.lang_unknown")}
              </h4>
              {list(items)}
            </section>
          ))}
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
