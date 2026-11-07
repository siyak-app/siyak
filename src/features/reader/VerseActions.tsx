import {
  BookmarkIcon,
  CheckIcon,
  Columns2Icon,
  CopyIcon,
  FolderPlusIcon,
  HashIcon,
  LanguagesIcon,
  type LucideIcon,
  NotebookPenIcon,
  PauseIcon,
  PlayIcon,
  QuoteIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { announce } from "@/lib/announce";
import type { VerseWithTranslations } from "@/lib/bindings";
import { fetchCitation } from "@/lib/queries";
import type { Settings } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { translationsText, verseReference } from "./verseCopy";
import { openNoteEditor } from "./verseSelection";

/** How long a pressed action shows its check mark. */
const COPIED_MS = 1500;

interface Action {
  id: string;
  icon: LucideIcon;
  label: string;
  /** Text to copy, if this action copies. */
  text?: () => string | Promise<string>;
  /** What the action does, if it does not copy. */
  run?: () => void;
  /** Filled icon: the action's state is on. */
  active?: boolean;
}

/**
 * Actions for one verse: bookmarking, notes, collections, comparing and
 * copying. `hover` shows icon buttons with tooltips while the verse is hovered
 * or focused; `labeled` always shows them with their labels (`reader.actions`,
 * spec §13.2). An action whose handler is not given is not offered.
 */
export function VerseActions({
  verse,
  surahName,
  titles,
  mode,
  onCompare,
  bookmarked = false,
  onBookmark,
  onPlay,
  playing = false,
}: {
  verse: VerseWithTranslations;
  /** The surah's name in the interface language, for the reference. */
  surahName: string;
  /** Title per shown translation pack id. */
  titles: Readonly<Record<string, string>>;
  mode: Settings["reader.actions"];
  /** Opens the verse on the compare screen. */
  onCompare?: (verse: VerseWithTranslations) => void;
  /** Whether the verse has a bookmark. */
  bookmarked?: boolean;
  /** Toggles the verse's bookmark. */
  onBookmark?: (verse: VerseWithTranslations) => void;
  /** Plays the verse's recitation, or pauses it; only with an installed reciter. */
  onPlay?: (verse: VerseWithTranslations) => void;
  /** Whether this verse's recitation is playing. */
  playing?: boolean;
}) {
  const { t } = useTranslation();
  const citationAction = useUiStore((state) => state.preferences["reader.citation_action"]);
  const citationStyle = useUiStore((state) => state.preferences["reader.citation_style"]);
  const openCollectionPicker = useUiStore((state) => state.openCollectionPicker);
  const [copied, setCopied] = useState<string | null>(null);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => {
      setCopied(null);
    }, COPIED_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [copied]);

  const actions: Action[] = [];
  if (onPlay) {
    actions.push({
      id: "play",
      icon: playing ? PauseIcon : PlayIcon,
      label: t(playing ? "verse_actions.pause" : "verse_actions.play"),
      active: playing,
      run: () => {
        onPlay(verse);
      },
    });
  }
  if (onBookmark) {
    actions.push({
      id: "bookmark",
      icon: BookmarkIcon,
      label: t(bookmarked ? "verse_actions.remove_bookmark" : "verse_actions.add_bookmark"),
      active: bookmarked,
      run: () => {
        onBookmark(verse);
      },
    });
    actions.push({
      id: "note",
      icon: NotebookPenIcon,
      label: t("verse_actions.note"),
      run: () => {
        openNoteEditor({ surah: verse.surah, ayah: verse.ayah });
      },
    });
    actions.push({
      id: "collection",
      icon: FolderPlusIcon,
      label: t("collections.add"),
      run: () => {
        openCollectionPicker({ kind: "verse_range", from: verse.id, to: verse.id });
      },
    });
  }
  if (onCompare) {
    actions.push({
      id: "compare",
      icon: Columns2Icon,
      label: t("verse_actions.compare"),
      run: () => {
        onCompare(verse);
      },
    });
  }
  actions.push({
    id: "copy_arabic",
    icon: CopyIcon,
    label: t("verse_actions.copy_arabic"),
    text: () => verse.text_uthmani,
  });
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
  if (citationAction) {
    actions.push({
      id: "copy_citation",
      icon: QuoteIcon,
      label: t("verse_actions.copy_citation"),
      text: () =>
        fetchCitation(
          {
            kind: "verses",
            from: verse.id,
            to: verse.id,
            translation_ids: verse.translations.map((translation) => translation.pack_id),
          },
          citationStyle,
        ),
    });
  }

  const run = async (action: Action) => {
    if (action.run) {
      action.run();
      return;
    }
    try {
      await navigator.clipboard.writeText((await action.text?.()) ?? "");
      setCopied(action.id);
      announce(t("verse_actions.copied"));
    } catch (error) {
      console.error("Copying to the clipboard failed", error);
      announce(t("verse_actions.copy_failed"));
    }
  };

  const iconOf = (action: Action) => {
    const Icon = copied === action.id ? CheckIcon : action.icon;
    return <Icon aria-hidden className={action.active ? "fill-current text-primary" : undefined} />;
  };

  if (mode === "labeled") {
    return (
      <div className="-ms-2 flex flex-wrap gap-1">
        {actions.map((action) => (
          <Button
            key={action.id}
            variant="ghost"
            size="xs"
            className="text-muted-foreground"
            onClick={() => void run(action)}
          >
            {iconOf(action)}
            {action.label}
          </Button>
        ))}
      </div>
    );
  }

  return (
    <div
      data-copied={copied !== null || undefined}
      className="grid grid-cols-2 gap-0.5 opacity-0 transition-opacity group-focus-within/verse:opacity-100 group-hover/verse:opacity-100 data-copied:opacity-100 motion-reduce:transition-none"
    >
      {actions.map((action) => (
        <Tooltip key={action.id}>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label={action.label}
              className="text-muted-foreground"
              onClick={() => void run(action)}
            >
              {iconOf(action)}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="left">{action.label}</TooltipContent>
        </Tooltip>
      ))}
    </div>
  );
}
