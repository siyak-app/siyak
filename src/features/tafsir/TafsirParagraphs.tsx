import type { TafsirInfo, TafsirPassage } from "@/lib/bindings";

import { pageMarks } from "./tafsirRefs";
import { TafsirText } from "./TafsirText";

/** The paragraphs of a passage in the tafsir's language and direction. */
export function TafsirParagraphs({
  tafsir,
  passage,
}: {
  tafsir: TafsirInfo;
  passage: TafsirPassage;
}) {
  const marks = pageMarks(passage.paragraphs);
  return (
    <div
      dir={tafsir.dir}
      lang={tafsir.lang ?? undefined}
      className="translation-text grid gap-3 text-start"
    >
      {passage.paragraphs.map((paragraph, index) => {
        const mark = marks[index];
        return (
          <p key={paragraph.idx}>
            <TafsirText html={paragraph.text} />
            {mark && (
              <span dir="ltr" className="ms-2 text-xs text-muted-foreground tabular-nums">
                [{mark}]
              </span>
            )}
          </p>
        );
      })}
    </div>
  );
}
