import { Columns2Icon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { packTitle } from "@/features/packs/packFormat";
import { ChooserRow } from "@/features/reader/TranslationPicker";
import { moveTranslation, toggleTranslation } from "@/features/reader/translationOrder";
import type { TafsirInfo, TranslationInfo } from "@/lib/bindings";

import type { CompareColumn } from "./compareLayout";

/** Adds, removes and reorders the columns of the compare table. */
export function ColumnPicker({
  columns,
  translations,
  tafsirs,
  onChange,
}: {
  columns: readonly CompareColumn[];
  translations: readonly TranslationInfo[];
  tafsirs: readonly TafsirInfo[];
  /** The new column ids, in order. */
  onChange: (ids: string[]) => void;
}) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  const shownIds = columns.map((column) => column.pack.id);
  const others = [...translations, ...tafsirs].filter((pack) => !shownIds.includes(pack.id));
  const title = (pack: TranslationInfo | TafsirInfo) =>
    packTitle(pack.titles, pack.title, language);

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm">
          <Columns2Icon aria-hidden />
          {t("compare.columns_button")}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <PopoverHeader>
          <PopoverTitle>{t("compare.columns_title")}</PopoverTitle>
          <PopoverDescription>{t("compare.columns_hint")}</PopoverDescription>
        </PopoverHeader>
        <ul className="-mx-1 flex max-h-80 flex-col overflow-y-auto">
          {columns.map(({ pack }, index) => (
            <ChooserRow
              key={pack.id}
              title={title(pack)}
              lang={pack.lang}
              checked
              first={index === 0}
              last={index === columns.length - 1}
              onCheckedChange={(checked) => {
                onChange(toggleTranslation(shownIds, pack.id, checked));
              }}
              onMove={(by) => {
                onChange(moveTranslation(shownIds, pack.id, by));
              }}
            />
          ))}
          {others.map((pack) => (
            <ChooserRow
              key={pack.id}
              title={title(pack)}
              lang={pack.lang}
              checked={false}
              onCheckedChange={(checked) => {
                onChange(toggleTranslation(shownIds, pack.id, checked));
              }}
            />
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}
