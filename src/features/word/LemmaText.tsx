import { splitLemma } from "./morphologyLabels";

/** A lemma in Arabic, with QAC's homograph index raised. */
export function LemmaText({ lemma }: { lemma: string }) {
  const { text, index } = splitLemma(lemma);
  return (
    <span dir="rtl" lang="ar">
      {text}
      {index !== null && <sup>{index}</sup>}
    </span>
  );
}
