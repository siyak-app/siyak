import { type KeyboardEvent, type RefObject, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { Note, VerseRef } from "@/lib/bindings";
import { useSurahs, useUpsertNote } from "@/lib/queries";

/** Writes a new note on `from` (and on through a later verse), or changes `note`. */
export function NoteEditor({
  from,
  fromId,
  note,
  onDone,
  textareaRef,
}: {
  /** The first verse. */
  from: VerseRef;
  /** Its global id. */
  fromId: number;
  /** The note to change, if not a new one. */
  note?: Note;
  /** Called after saving, and on cancelling an edit. */
  onDone?: () => void;
  /** To focus the text from outside (the `n` key). */
  textareaRef?: RefObject<HTMLTextAreaElement | null>;
}) {
  const { t } = useTranslation();
  const [body, setBody] = useState(note?.body ?? "");
  const [through, setThrough] = useState(note?.to.ayah ?? from.ayah);
  const surahs = useSurahs();
  const upsert = useUpsertNote();
  const bodyId = useId();
  const throughId = useId();
  const lastAyah = surahs.data?.find((surah) => surah.number === from.surah)?.ayah_count;
  const blank = body.trim() === "";

  const save = () => {
    if (blank || upsert.isPending) return;
    // Global ids are consecutive within a surah (spec §6.2).
    const to = Math.max(through, from.ayah);
    upsert.mutate(
      {
        id: note?.id ?? null,
        verse_from: fromId,
        verse_to: fromId + to - from.ayah,
        body,
      },
      {
        onSuccess: () => {
          setBody("");
          setThrough(from.ayah);
          onDone?.();
        },
      },
    );
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      save();
    }
  };

  return (
    <div className="grid gap-2">
      <label htmlFor={bodyId} className="text-xs font-medium">
        {note ? t("notes.body") : t("notes.new")}
      </label>
      <Textarea
        id={bodyId}
        ref={textareaRef}
        rows={4}
        dir="auto"
        value={body}
        placeholder={t("notes.placeholder")}
        onChange={(event) => {
          setBody(event.target.value);
        }}
        onKeyDown={onKeyDown}
      />
      <div className="flex flex-wrap items-center gap-2">
        <label htmlFor={throughId} className="text-xs text-muted-foreground">
          {t("notes.through")}
        </label>
        <Input
          id={throughId}
          type="number"
          min={from.ayah}
          max={lastAyah}
          value={through}
          className="h-7 w-20"
          onChange={(event) => {
            const value = Number(event.target.value);
            if (Number.isInteger(value)) setThrough(value);
          }}
        />
        <span className="flex-1" />
        {note && (
          <Button variant="ghost" size="sm" onClick={onDone}>
            {t("notes.cancel")}
          </Button>
        )}
        <Button size="sm" disabled={blank || upsert.isPending} onClick={save}>
          {t("notes.save")}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">{t("notes.save_hint")}</p>
      {upsert.isError && <ErrorMessage error={upsert.error} />}
    </div>
  );
}
