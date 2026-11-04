import { FolderPlusIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { verseIdOf } from "@/features/tafsir/tafsirRefs";
import type { ItemRef } from "@/lib/bindings";
import { useSurahs } from "@/lib/queries";
import type { VersePosition } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/** Opens the "Add to collection" dialog for `item`. */
export function AddToCollectionButton({ item, label }: { item: ItemRef; label?: string }) {
  const { t } = useTranslation();
  const open = useUiStore((state) => state.openCollectionPicker);
  return (
    <Button
      variant="ghost"
      size="xs"
      className="text-muted-foreground"
      onClick={() => {
        open(item);
      }}
    >
      <FolderPlusIcon aria-hidden />
      {label ?? t("collections.add")}
    </Button>
  );
}

/**
 * The same for a verse known by its position, e.g. a result in a list; with
 * `through`, for the verses up to that one. With a `label` it shows its text,
 * otherwise it is a small icon that sits on the result.
 */
export function AddVerseButton({
  verse,
  through,
  label,
  className,
}: {
  verse: VersePosition;
  through?: number;
  label?: string;
  className?: string;
}) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  const open = useUiStore((state) => state.openCollectionPicker);
  if (!surahs.data) return null;
  const from = verseIdOf(surahs.data, verse);
  const onClick = () => {
    open({ kind: "verse_range", from, to: from + (through ?? verse.ayah) - verse.ayah });
  };
  if (label !== undefined) {
    return (
      <Button variant="ghost" size="sm" className={className} onClick={onClick}>
        <FolderPlusIcon aria-hidden />
        {label}
      </Button>
    );
  }
  const name = t("collections.add");
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label={name}
          className={`z-10 text-muted-foreground ${className ?? ""}`}
          onClick={onClick}
        >
          <FolderPlusIcon aria-hidden />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{name}</TooltipContent>
    </Tooltip>
  );
}
