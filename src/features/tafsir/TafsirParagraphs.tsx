import type { ReactNode } from "react";

import type { TafsirInfo, TafsirPassage } from "@/lib/bindings";

import { pageMarks } from "./tafsirRefs";
import { TafsirText } from "./TafsirText";

/**
 * The paragraphs of a passage in the tafsir's language and direction.
 * `after` renders what follows a paragraph (its machine translation, spec §9.3),
 * and `highlighted` marks one paragraph, e.g. the original of a translation.
 */
export function TafsirParagraphs({
  tafsir,
  passage,
  after,
  highlighted = null,
}: {
  tafsir: TafsirInfo;
  passage: TafsirPassage;
  after?: (idx: number) => ReactNode;
  highlighted?: number | null;
}) {
  const marks = pageMarks(passage.paragraphs);
  return (
    <div className="grid gap-3">
      {passage.paragraphs.map((paragraph, index) => {
        const mark = marks[index];
        const isHighlighted = highlighted === paragraph.idx;
        return (
          <div key={paragraph.idx} className="grid gap-2">
            <p
              dir={tafsir.dir}
              lang={tafsir.lang ?? undefined}
              data-highlighted={isHighlighted || undefined}
              ref={(node) => {
                if (isHighlighted) node?.scrollIntoView({ block: "nearest", behavior: "smooth" });
              }}
              className="translation-text rounded-sm text-start transition-colors data-highlighted:bg-accent"
            >
              <TafsirText html={paragraph.text} />
              {mark && (
                <span dir="ltr" className="ms-2 text-xs text-muted-foreground tabular-nums">
                  [{mark}]
                </span>
              )}
            </p>
            {after?.(paragraph.idx)}
          </div>
        );
      })}
    </div>
  );
}
