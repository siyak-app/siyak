import { ChevronDownIcon, ChevronUpIcon, LanguagesIcon } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import type { TranslationInfo } from "@/lib/bindings";
import { useTranslations } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { moveTranslation, toggleTranslation } from "./translationOrder";
import { translationTitle } from "./translationTitle";

/** A language code as a name in the interface language, e.g. `ur` → "Urdu". */
function languageName(code: string, language: string | undefined): string {
  try {
    return new Intl.DisplayNames(language, { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

/**
 * Chooses the translations shown in the reader and their order
 * (`reader.translations`). Shown translations come first, in display order,
 * each with move up/down buttons; the other installed ones follow.
 */
export function TranslationPicker() {
  const { t } = useTranslation();
  const installed = useTranslations();
  const selected = useUiStore((state) => state.preferences["reader.translations"]);
  const setPreference = useUiStore((state) => state.setPreference);

  const packs = installed.data ?? [];
  const byId = new Map(packs.map((pack) => [pack.id, pack]));
  // Ids of packs that are no longer installed are dropped on the next change.
  const shown = selected.flatMap((id) => byId.get(id) ?? []);
  const shownIds = shown.map((pack) => pack.id);
  const others = packs.filter((pack) => !shownIds.includes(pack.id));

  const update = (next: string[]) => {
    setPreference("reader.translations", next);
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm">
          <LanguagesIcon aria-hidden />
          {t("reader.translations_button")}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <PopoverHeader>
          <PopoverTitle>{t("reader.translations_picker_title")}</PopoverTitle>
          <PopoverDescription>{t("reader.translations_picker_hint")}</PopoverDescription>
        </PopoverHeader>
        {packs.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("reader.no_translations_installed")}</p>
        ) : (
          <ul className="-mx-1 flex flex-col">
            {shown.map((pack, index) => (
              <TranslationRow
                key={pack.id}
                pack={pack}
                checked
                first={index === 0}
                last={index === shown.length - 1}
                onCheckedChange={(checked) => {
                  update(toggleTranslation(shownIds, pack.id, checked));
                }}
                onMove={(by) => {
                  update(moveTranslation(shownIds, pack.id, by));
                }}
              />
            ))}
            {others.map((pack) => (
              <TranslationRow
                key={pack.id}
                pack={pack}
                checked={false}
                onCheckedChange={(checked) => {
                  update(toggleTranslation(shownIds, pack.id, checked));
                }}
              />
            ))}
          </ul>
        )}
      </PopoverContent>
    </Popover>
  );
}

function TranslationRow({
  pack,
  checked,
  first = false,
  last = false,
  onCheckedChange,
  onMove,
}: {
  pack: TranslationInfo;
  checked: boolean;
  first?: boolean;
  last?: boolean;
  onCheckedChange: (checked: boolean) => void;
  onMove?: (by: -1 | 1) => void;
}) {
  const { t, i18n } = useTranslation();
  const id = useId();
  const title = translationTitle(pack, i18n.resolvedLanguage);
  return (
    <li className="flex items-center gap-2 rounded-md px-1 py-1.5">
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(value) => {
          onCheckedChange(value === true);
        }}
      />
      <label htmlFor={id} className="min-w-0 flex-1 text-sm">
        <span className="block truncate">{title}</span>
        {pack.lang && (
          <span className="block text-xs text-muted-foreground">
            {languageName(pack.lang, i18n.resolvedLanguage)}
          </span>
        )}
      </label>
      {onMove && (
        <span className="flex">
          {/* aria-disabled rather than disabled, so focus stays on the button
              after it moves its translation to either end. */}
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label={t("reader.move_translation_up", { title })}
            aria-disabled={first}
            className="aria-disabled:opacity-50"
            onClick={() => {
              if (!first) onMove(-1);
            }}
          >
            <ChevronUpIcon aria-hidden />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label={t("reader.move_translation_down", { title })}
            aria-disabled={last}
            className="aria-disabled:opacity-50"
            onClick={() => {
              if (!last) onMove(1);
            }}
          >
            <ChevronDownIcon aria-hidden />
          </Button>
        </span>
      )}
    </li>
  );
}
