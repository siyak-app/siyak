import { Link } from "@tanstack/react-router";
import { CheckIcon } from "lucide-react";
import { type SubmitEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { verseOfId } from "@/features/tafsir/tafsirRefs";
import { announce } from "@/lib/announce";
import type { ItemRef } from "@/lib/bindings";
import {
  useAddCollectionItem,
  useCollections,
  useSurahs,
  useUpsertCollection,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { namedRange } from "./itemLabels";

/** What is being added, in words. */
function useItemLabel(item: ItemRef): string {
  const { t, i18n } = useTranslation();
  const surahs = useSurahs();
  switch (item.kind) {
    case "verse_range": {
      const from = surahs.data && verseOfId(surahs.data, item.from);
      const to = surahs.data && verseOfId(surahs.data, item.to);
      return from && to ? namedRange(surahs.data, from, to, i18n.resolvedLanguage) : "";
    }
    case "note":
      return t("collections.note_item");
    case "root":
      return item.root;
    case "lemma":
      return item.lemma.replace(/\d+$/u, "");
    case "tafsir": {
      const verse = surahs.data && verseOfId(surahs.data, item.verse);
      const place = verse ? namedRange(surahs.data, verse, verse, i18n.resolvedLanguage) : "";
      return `${t("collections.tafsir_item")} · ${place}`;
    }
  }
}

/** Mounted once in the shell: any screen opens it with `openCollectionPicker`. */
export function AddToCollectionDialog() {
  const item = useUiStore((state) => state.collectionPicker);
  const open = useUiStore((state) => state.openCollectionPicker);
  return (
    <Dialog
      open={item !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) open(null);
      }}
    >
      <DialogContent className="flex max-h-[85vh] flex-col">
        {item && <Picker key={JSON.stringify(item)} item={item} />}
      </DialogContent>
    </Dialog>
  );
}

function Picker({ item }: { item: ItemRef }) {
  const { t } = useTranslation();
  const close = useUiStore((state) => state.openCollectionPicker);
  const [annotation, setAnnotation] = useState("");
  const [title, setTitle] = useState("");
  const [through, setThrough] = useState<number | null>(null);
  const [added, setAdded] = useState<{ id: number; title: string } | null>(null);
  const collections = useCollections();
  const surahs = useSurahs();
  const addItem = useAddCollectionItem();
  const create = useUpsertCollection();
  const annotationId = useId();
  const titleId = useId();
  const throughId = useId();

  const first =
    item.kind === "verse_range" && surahs.data ? verseOfId(surahs.data, item.from) : null;
  const last = item.kind === "verse_range" && surahs.data ? verseOfId(surahs.data, item.to) : null;
  const lastAyah = first
    ? surahs.data?.find((surah) => surah.number === first.surah)?.ayah_count
    : 0;
  const reference: ItemRef =
    item.kind === "verse_range" && first && through !== null
      ? { ...item, to: item.from + Math.max(through, first.ayah) - first.ayah }
      : item;
  const label = useItemLabel(reference);

  const addTo = (id: number, collectionTitle: string) => {
    addItem.mutate(
      { collectionId: id, item: reference, annotation: annotation.trim() || null },
      {
        onSuccess: () => {
          setAdded({ id, title: collectionTitle });
          announce(t("collections.added", { title: collectionTitle }));
        },
      },
    );
  };

  const createAndAdd = (event: SubmitEvent) => {
    event.preventDefault();
    if (title.trim() === "") return;
    create.mutate(
      { id: null, title, description: null },
      {
        onSuccess: (collection) => {
          addTo(collection.id, collection.title);
        },
      },
    );
  };

  if (added) {
    return (
      <>
        <DialogHeader>
          <DialogTitle>{t("collections.add")}</DialogTitle>
          <DialogDescription>{label}</DialogDescription>
        </DialogHeader>
        <p role="status" className="flex items-center gap-2 text-sm">
          <CheckIcon aria-hidden className="size-4 text-primary" />
          {t("collections.added", { title: added.title })}
        </p>
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={() => {
              close(null);
            }}
          >
            {t("collections.close")}
          </Button>
          <Button asChild>
            <Link
              to="/collections/$id"
              params={{ id: added.id }}
              onClick={() => {
                close(null);
              }}
            >
              {t("collections.open")}
            </Link>
          </Button>
        </DialogFooter>
      </>
    );
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t("collections.add")}</DialogTitle>
        <DialogDescription>{t("collections.adding", { what: label })}</DialogDescription>
      </DialogHeader>
      <div className="grid min-h-0 gap-4 overflow-y-auto p-0.5">
        {first && last?.surah === first.surah && (
          <div className="flex items-center gap-2">
            <label htmlFor={throughId} className="text-sm">
              {t("notes.through")}
            </label>
            <Input
              id={throughId}
              type="number"
              min={first.ayah}
              max={lastAyah}
              value={through ?? last.ayah}
              className="h-8 w-24"
              onChange={(event) => {
                const value = Number(event.target.value);
                if (Number.isInteger(value)) setThrough(value);
              }}
            />
          </div>
        )}
        <div className="grid gap-1.5">
          <label htmlFor={annotationId} className="text-sm">
            {t("collections.annotation")}
          </label>
          <Input
            id={annotationId}
            value={annotation}
            onChange={(event) => {
              setAnnotation(event.target.value);
            }}
          />
        </div>
        {collections.isError && <ErrorMessage error={collections.error} />}
        {collections.data?.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("collections.none_yet")}</p>
        ) : (
          <div className="grid gap-1.5">
            <p className="text-sm font-medium">{t("collections.choose")}</p>
            <ul className="grid max-h-56 gap-1 overflow-y-auto">
              {collections.data?.map((collection) => (
                <li key={collection.id}>
                  <Button
                    variant="outline"
                    className="w-full justify-between"
                    disabled={addItem.isPending}
                    onClick={() => {
                      addTo(collection.id, collection.title);
                    }}
                  >
                    <span className="truncate">{collection.title}</span>
                    <span className="text-xs text-muted-foreground">
                      {t("collections.item_count", { count: collection.item_count })}
                    </span>
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        )}
        <form onSubmit={createAndAdd} className="grid gap-1.5">
          <label htmlFor={titleId} className="text-sm">
            {t("collections.new_title")}
          </label>
          <div className="flex gap-2">
            <Input
              id={titleId}
              value={title}
              onChange={(event) => {
                setTitle(event.target.value);
              }}
            />
            <Button
              type="submit"
              variant="secondary"
              disabled={title.trim() === "" || create.isPending || addItem.isPending}
            >
              {t("collections.create_and_add")}
            </Button>
          </div>
        </form>
        {addItem.isError && <ErrorMessage error={addItem.error} />}
        {create.isError && <ErrorMessage error={create.error} />}
      </div>
    </>
  );
}
