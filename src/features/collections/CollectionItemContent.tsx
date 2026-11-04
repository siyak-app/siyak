import { Link } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";

import { Term } from "@/components/Term";
import { NoteMarkdown } from "@/features/notes/NoteMarkdown";
import { packTitle } from "@/features/packs/packFormat";
import { TafsirParagraphs } from "@/features/tafsir/TafsirParagraphs";
import { LemmaText } from "@/features/word/LemmaText";
import { MorphologySource } from "@/features/word/MorphologySource";
import type { CollectionItem, ItemContent, VerseRef } from "@/lib/bindings";
import { useSurahs } from "@/lib/queries";

import { namedRange } from "./itemLabels";
import { VerseText } from "./VerseText";

function Missing({ children }: { children: string }) {
  return <p className="text-sm text-muted-foreground">{children}</p>;
}

function VerseLink({ from, to }: { from: VerseRef; to: VerseRef }) {
  const { i18n } = useTranslation();
  const surahs = useSurahs();
  return (
    <Link
      to="/read/$surah/$ayah"
      params={from}
      className="text-sm font-medium underline-offset-2 hover:underline"
    >
      {namedRange(surahs.data, from, to, i18n.resolvedLanguage)}
    </Link>
  );
}

/** The content of one item, as it is now. */
export function CollectionItemContent({ item }: { item: CollectionItem }) {
  return <Content item={item} content={item.content} />;
}

function Content({ item, content }: { item: CollectionItem; content: ItemContent }) {
  const { t, i18n } = useTranslation();
  switch (content.kind) {
    case "verse_range": {
      const first = content.verses[0];
      const last = content.verses.at(-1);
      if (!first || !last) return null;
      return (
        <div className="grid gap-2">
          <VerseLink from={first} to={last} />
          <div className="grid gap-3">
            {content.verses.map((verse) => (
              <VerseText key={verse.id} verse={verse} />
            ))}
          </div>
        </div>
      );
    }
    case "note":
      if (!content.note) return <Missing>{t("collections.note_gone")}</Missing>;
      return (
        <div className="grid gap-2">
          <VerseLink from={content.note.from} to={content.note.to} />
          <NoteMarkdown>{content.note.body}</NoteMarkdown>
        </div>
      );
    case "root": {
      const text = item.item.kind === "root" ? item.item.root : "";
      return (
        <div className="grid gap-1">
          <p className="text-xs text-muted-foreground first-letter:uppercase">
            <Term name="root" />
          </p>
          <p dir="rtl" lang="ar" className="text-start font-quran text-2xl leading-loose">
            {text}
          </p>
          {content.root ? (
            <>
              <Link
                to="/concordance/$kind/$id"
                params={{ kind: "root", id: content.root.id }}
                className="text-sm underline underline-offset-2"
              >
                {t("word.root_occurrences", { count: content.root.occurrences })}
              </Link>
              <MorphologySource />
            </>
          ) : (
            <Missing>{t("collections.unavailable")}</Missing>
          )}
        </div>
      );
    }
    case "lemma": {
      const text = item.item.kind === "lemma" ? item.item.lemma : "";
      return (
        <div className="grid gap-1">
          <p className="text-xs text-muted-foreground first-letter:uppercase">
            <Term name="lemma" />
          </p>
          <p dir="rtl" lang="ar" className="text-start font-quran text-2xl leading-loose">
            <LemmaText lemma={text} />
          </p>
          {content.lemma ? (
            <>
              <Link
                to="/concordance/$kind/$id"
                params={{ kind: "lemma", id: content.lemma.id }}
                className="text-sm underline underline-offset-2"
              >
                {t("word.lemma_occurrences", { count: content.lemma.occurrences })}
              </Link>
              <MorphologySource />
            </>
          ) : (
            <Missing>{t("collections.unavailable")}</Missing>
          )}
        </div>
      );
    }
    case "tafsir":
      if (!content.tafsir || !content.passage) {
        return (
          <div className="grid gap-1">
            {content.verse && <VerseLink from={content.verse} to={content.verse} />}
            <Missing>{t("collections.unavailable")}</Missing>
          </div>
        );
      }
      return (
        <div className="grid gap-2">
          <p className="text-sm">
            <span className="font-medium">
              {packTitle(content.tafsir.titles, content.tafsir.title, i18n.resolvedLanguage)}
            </span>
            {" · "}
            <VerseLink from={content.passage.verse_from} to={content.passage.verse_to} />
          </p>
          <div className="max-h-72 overflow-y-auto rounded-md">
            <TafsirParagraphs tafsir={content.tafsir} passage={content.passage} />
          </div>
          {content.tafsir.attribution && (
            <p className="text-xs text-muted-foreground">{content.tafsir.attribution}</p>
          )}
        </div>
      );
  }
}
