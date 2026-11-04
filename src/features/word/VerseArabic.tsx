import { Fragment, type MouseEvent, useState } from "react";

import { Popover, PopoverAnchor, PopoverContent } from "@/components/ui/popover";
import { hasTextSelection, openContextPanel } from "@/features/reader/verseSelection";
import type { VerseWithTranslations } from "@/lib/bindings";
import { useUiStore } from "@/stores/ui";

import { WordCard } from "./WordCard";

/**
 * A verse's Arabic text. With a morphology pack every word can be clicked
 * (spec §5.3). The words are spans joined by plain spaces rather than buttons,
 * so the text selects, copies and breaks across lines exactly like the plain
 * verse. Keyboard users reach them through the Word tab.
 */
export function VerseArabic({ verse }: { verse: VerseWithTranslations }) {
  const underline = useUiStore((state) => state.preferences["word.underline"]);
  const clickMode = useUiStore((state) => state.preferences["word.click"]);
  const selectWord = useUiStore((state) => state.selectWord);
  const selectedHere = useUiStore((state) =>
    state.selectedVerse?.surah === verse.surah && state.selectedVerse.ayah === verse.ayah
      ? state.selectedWord
      : null,
  );
  const [card, setCard] = useState<{ wordId: number; element: HTMLElement } | null>(null);

  if (verse.words.length === 0) {
    return (
      <p dir="rtl" lang="ar" className="arabic-text">
        {verse.text_uthmani}
      </p>
    );
  }

  const onWordClick = (event: MouseEvent<HTMLElement>, wordId: number) => {
    if (hasTextSelection()) return;
    selectWord({ surah: verse.surah, ayah: verse.ayah }, wordId);
    if (clickMode === "panel") openContextPanel("word");
    else setCard({ wordId, element: event.currentTarget });
  };

  return (
    <p dir="rtl" lang="ar" className="arabic-text">
      {verse.words.map((word, index) => (
        <Fragment key={word.id}>
          {index > 0 && " "}
          <span
            role="button"
            data-word={word.id}
            data-selected={word.id === selectedHere || undefined}
            className={`cursor-pointer rounded-sm transition-colors hover:bg-primary/10 data-selected:bg-primary/15 ${
              underline === "dotted" ? "underline decoration-dotted underline-offset-8" : ""
            }`}
            onClick={(event) => {
              onWordClick(event, word.id);
            }}
          >
            {word.text_ar}
          </span>
        </Fragment>
      ))}
      {card && (
        <Popover
          open
          onOpenChange={(open) => {
            if (!open) setCard(null);
          }}
        >
          <PopoverAnchor virtualRef={{ current: card.element }} />
          <PopoverContent
            dir="ltr"
            lang={undefined}
            onCloseAutoFocus={(event) => {
              event.preventDefault();
            }}
            // React events bubble through the portal to the verse list.
            onClick={(event) => {
              event.stopPropagation();
            }}
          >
            <WordCard
              wordId={card.wordId}
              onDetails={() => {
                setCard(null);
                openContextPanel("word");
              }}
            />
          </PopoverContent>
        </Popover>
      )}
    </p>
  );
}
