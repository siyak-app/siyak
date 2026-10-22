import { CheckIcon, CopyIcon, HashIcon, LanguagesIcon, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { announce } from "@/lib/announce";
import type { VerseWithTranslations } from "@/lib/bindings";
import type { Settings } from "@/lib/settings";

import { translationsText, verseReference } from "./verseCopy";

/** How long a pressed action shows its check mark. */
const COPIED_MS = 1500;

type ActionId = "copy_arabic" | "copy_translation" | "copy_reference";

interface Action {
  id: ActionId;
  icon: LucideIcon;
  label: string;
  text: () => string;
}

/**
 * Copy actions for one verse. `hover` shows icon buttons with tooltips while
 * the verse is hovered or focused; `labeled` always shows them with their
 * labels (`reader.actions`, spec §13.2). Only actions that exist today are
 * offered.
 */
export function VerseActions({
  verse,
  surahName,
  titles,
  mode,
}: {
  verse: VerseWithTranslations;
  /** The surah's name in the interface language, for the reference. */
  surahName: string;
  /** Title per shown translation pack id. */
  titles: Readonly<Record<string, string>>;
  mode: Settings["reader.actions"];
}) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState<ActionId | null>(null);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => {
      setCopied(null);
    }, COPIED_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);

  const actions: Action[] = [
    {
      id: "copy_arabic",
      icon: CopyIcon,
      label: t("verse_actions.copy_arabic"),
      text: () => verse.text_uthmani,
    },
  ];
  if (verse.translations.length > 0) {
    actions.push({
      id: "copy_translation",
      icon: LanguagesIcon,
      label: t("verse_actions.copy_translation", { count: verse.translations.length }),
      text: () => translationsText(verse, titles),
    });
  }
  actions.push({
    id: "copy_reference",
    icon: HashIcon,
    label: t("verse_actions.copy_reference"),
    text: () => verseReference(surahName, verse),
  });

  const run = async (action: Action) => {
    try {
      await navigator.clipboard.writeText(action.text());
      setCopied(action.id);
      announce(t("verse_actions.copied"));
    } catch (error) {
      console.error("Copying to the clipboard failed", error);
      announce(t("verse_actions.copy_failed"));
    }
  };

  if (mode === "labeled") {
    return (
      <div className="-ms-2 flex flex-wrap gap-1">
        {actions.map((action) => {
          const Icon = copied === action.id ? CheckIcon : action.icon;
          return (
            <Button
              key={action.id}
              variant="ghost"
              size="xs"
              className="text-muted-foreground"
              onClick={() => void run(action)}
            >
              <Icon aria-hidden />
              {action.label}
            </Button>
          );
        })}
      </div>
    );
  }

  return (
    <div
      data-copied={copied !== null || undefined}
      className="flex flex-col gap-0.5 opacity-0 transition-opacity group-focus-within/verse:opacity-100 group-hover/verse:opacity-100 data-copied:opacity-100 motion-reduce:transition-none"
    >
      {actions.map((action) => {
        const Icon = copied === action.id ? CheckIcon : action.icon;
        return (
          <Tooltip key={action.id}>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={action.label}
                className="text-muted-foreground"
                onClick={() => void run(action)}
              >
                <Icon aria-hidden />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="left">{action.label}</TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
}
