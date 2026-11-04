import { Link } from "@tanstack/react-router";
import { PencilIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ConfirmButton } from "@/components/ConfirmButton";
import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { AddToCollectionButton } from "@/features/collections/AddToCollectionButton";
import { namedRange } from "@/features/collections/itemLabels";
import type { Note } from "@/lib/bindings";
import { useDeleteNote, useSurahs } from "@/lib/queries";

import { NoteEditor } from "./NoteEditor";
import { NoteMarkdown } from "./NoteMarkdown";

/** One note: its verses, its text and what can be done with it. */
export function NoteCard({ note }: { note: Note }) {
  const { t, i18n } = useTranslation();
  const [editing, setEditing] = useState(false);
  const surahs = useSurahs();
  const remove = useDeleteNote();
  const reference = namedRange(surahs.data, note.from, note.to, i18n.resolvedLanguage);
  if (editing) {
    return (
      <div className="rounded-lg border p-2.5">
        <NoteEditor
          from={note.from}
          fromId={note.verse_from}
          note={note}
          onDone={() => {
            setEditing(false);
          }}
        />
      </div>
    );
  }
  return (
    <article aria-label={reference} className="grid gap-2 rounded-lg border p-2.5">
      <Link
        to="/read/$surah/$ayah"
        params={note.from}
        aria-label={t("notes.open_verse", { reference })}
        className="text-xs text-muted-foreground underline-offset-2 hover:underline"
      >
        {reference}
      </Link>
      <NoteMarkdown>{note.body}</NoteMarkdown>
      <div className="-ms-2 flex flex-wrap gap-1">
        <Button
          variant="ghost"
          size="xs"
          className="text-muted-foreground"
          onClick={() => {
            setEditing(true);
          }}
        >
          <PencilIcon aria-hidden />
          {t("notes.edit")}
        </Button>
        <AddToCollectionButton item={{ kind: "note", id: note.id }} />
        <ConfirmButton
          icon={Trash2Icon}
          label={t("notes.delete")}
          confirmLabel={t("notes.delete_confirm")}
          onConfirm={() => {
            remove.mutate(note.id);
          }}
        />
      </div>
      {remove.isError && <ErrorMessage error={remove.error} />}
    </article>
  );
}
