import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { useNotes } from "@/lib/queries";

import { NoteCard } from "./NoteCard";

/** All notes, the most recently changed first. */
export function NotesView() {
  const { t } = useTranslation();
  const notes = useNotes(null);
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-8">
        <h1 className="text-lg font-semibold">{t("notes.title")}</h1>
        {notes.isError && <ErrorMessage error={notes.error} />}
        {notes.data?.length === 0 && <p className="text-muted-foreground">{t("notes.empty")}</p>}
        <ul className="grid gap-3">
          {notes.data?.map((note) => (
            <li key={note.id}>
              <NoteCard note={note} />
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
