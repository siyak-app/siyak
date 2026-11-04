import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { verseIdOf } from "@/features/tafsir/tafsirRefs";
import { useNotes, useSurahs } from "@/lib/queries";
import type { VersePosition } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { NoteCard } from "./NoteCard";
import { NoteEditor } from "./NoteEditor";

/** The Notes tab (spec §13.2): the notes touching the selected verse and a new one. */
export function NotesPanel({ verse }: { verse: VersePosition | null }) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  if (surahs.isError) return <ErrorMessage error={surahs.error} />;
  if (!verse || !surahs.data) {
    return <p className="p-1 pt-3 text-sm text-muted-foreground">{t("notes.select_verse")}</p>;
  }
  const verseId = verseIdOf(surahs.data, verse);
  return <VerseNotes key={verseId} verse={verse} verseId={verseId} />;
}

function VerseNotes({ verse, verseId }: { verse: VersePosition; verseId: number }) {
  const { t } = useTranslation();
  const notes = useNotes({ from: verseId, to: verseId });
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pending = useUiStore((state) => state.noteFocusPending);
  const setPending = useUiStore((state) => state.setNoteFocusPending);

  useEffect(() => {
    if (!pending) return;
    textareaRef.current?.focus();
    setPending(false);
  }, [pending, setPending]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-y-auto pt-2">
      {notes.isError && <ErrorMessage error={notes.error} />}
      {notes.data?.length === 0 && (
        <p className="p-1 text-sm text-muted-foreground">{t("notes.none_for_verse")}</p>
      )}
      {notes.data && notes.data.length > 0 && (
        <ul aria-label={t("notes.label")} className="grid gap-2">
          {notes.data.map((note) => (
            <li key={note.id}>
              <NoteCard note={note} />
            </li>
          ))}
        </ul>
      )}
      <NoteEditor from={verse} fromId={verseId} textareaRef={textareaRef} />
    </div>
  );
}
