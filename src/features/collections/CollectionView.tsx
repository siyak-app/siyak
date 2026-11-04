import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Link, useNavigate } from "@tanstack/react-router";
import {
  ArrowDownIcon,
  ArrowLeftIcon,
  ArrowUpIcon,
  FileDownIcon,
  GripVerticalIcon,
  PencilIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { ConfirmButton } from "@/components/ConfirmButton";
import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { NoteMarkdown } from "@/features/notes/NoteMarkdown";
import type { CollectionItem, CollectionSummary, MarkdownLabels } from "@/lib/bindings";
import {
  appErrorCode,
  useCollection,
  useDeleteCollection,
  useExportCollectionMarkdown,
  useMoveCollectionItem,
  useRemoveCollectionItem,
  useUpdateCollectionItem,
  useUpsertCollection,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { CollectionItemContent } from "./CollectionItemContent";
import { useItemName } from "./itemLabels";

/** One collection: its items in the user's order, and exporting it (spec §13.3). */
export function CollectionView({ id }: { id: number }) {
  const { t } = useTranslation();
  const translations = useUiStore((state) => state.preferences["reader.translations"]);
  const collection = useCollection(id, translations);

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-8">
        <Link
          to="/collections"
          className="flex items-center gap-1 text-sm text-muted-foreground underline-offset-2 hover:underline"
        >
          <ArrowLeftIcon aria-hidden className="size-3.5" />
          {t("collections.back")}
        </Link>
        {collection.isError &&
          (appErrorCode(collection.error) === "not_found" ? (
            <p className="text-muted-foreground">{t("collections.empty")}</p>
          ) : (
            <ErrorMessage error={collection.error} />
          ))}
        {collection.isPending && <p className="text-muted-foreground">{t("common.loading")}</p>}
        {collection.data && (
          <>
            <Header summary={collection.data.summary} />
            {collection.data.items.length === 0 ? (
              <p className="text-muted-foreground">{t("collections.no_items")}</p>
            ) : (
              <Items collectionId={id} items={collection.data.items} />
            )}
          </>
        )}
      </div>
    </div>
  );
}

function useMarkdownLabels(): MarkdownLabels {
  const { t, i18n } = useTranslation();
  return {
    language: i18n.resolvedLanguage ?? "en",
    notes: t("markdown.notes"),
    bibliography: t("markdown.bibliography"),
    root: t("term.root.plain"),
    lemma: t("term.lemma.plain"),
    occurrences: t("markdown.occurrences"),
    source: t("markdown.source"),
    unavailable: t("collections.unavailable"),
    quran_text: t("markdown.quran_text"),
  };
}

function Header({ summary }: { summary: CollectionSummary }) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const navigate = useNavigate();
  const remove = useDeleteCollection();
  const exportMarkdown = useExportCollectionMarkdown();
  const translations = useUiStore((state) => state.preferences["reader.translations"]);
  const style = useUiStore((state) => state.preferences["reader.citation_style"]);
  const labels = useMarkdownLabels();

  if (editing) {
    return (
      <EditForm
        summary={summary}
        onDone={() => {
          setEditing(false);
        }}
      />
    );
  }
  return (
    <header className="grid gap-2">
      <h1 className="text-lg font-semibold">{summary.title}</h1>
      {summary.description && <NoteMarkdown>{summary.description}</NoteMarkdown>}
      <div className="-ms-2 flex flex-wrap items-center gap-1">
        <Button
          variant="ghost"
          size="xs"
          className="text-muted-foreground"
          disabled={exportMarkdown.isPending}
          onClick={() => {
            exportMarkdown.mutate({
              id: summary.id,
              translationIds: translations,
              labels,
              style,
            });
          }}
        >
          <FileDownIcon aria-hidden />
          {t("collections.export")}
        </Button>
        <Button
          variant="ghost"
          size="xs"
          className="text-muted-foreground"
          onClick={() => {
            setEditing(true);
          }}
        >
          <PencilIcon aria-hidden />
          {t("collections.rename")}
        </Button>
        <ConfirmButton
          icon={Trash2Icon}
          label={t("collections.delete")}
          confirmLabel={t("collections.delete_confirm")}
          onConfirm={() => {
            remove.mutate(summary.id, {
              onSuccess: () => {
                void navigate({ to: "/collections" });
              },
            });
          }}
        />
      </div>
      {exportMarkdown.data && (
        <p role="status" className="text-xs wrap-anywhere text-muted-foreground">
          {t("collections.exported", { path: exportMarkdown.data })}
        </p>
      )}
      {exportMarkdown.isError && <ErrorMessage error={exportMarkdown.error} />}
      {remove.isError && <ErrorMessage error={remove.error} />}
    </header>
  );
}

function EditForm({ summary, onDone }: { summary: CollectionSummary; onDone: () => void }) {
  const { t } = useTranslation();
  const [title, setTitle] = useState(summary.title);
  const [description, setDescription] = useState(summary.description ?? "");
  const save = useUpsertCollection();
  const titleId = useId();
  const descriptionId = useId();
  return (
    <form
      className="grid gap-2"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate({ id: summary.id, title, description }, { onSuccess: onDone });
      }}
    >
      <label htmlFor={titleId} className="text-sm">
        {t("collections.name")}
      </label>
      <Input
        id={titleId}
        value={title}
        onChange={(event) => {
          setTitle(event.target.value);
        }}
      />
      <label htmlFor={descriptionId} className="text-sm">
        {t("collections.description")}
      </label>
      <Textarea
        id={descriptionId}
        rows={3}
        value={description}
        onChange={(event) => {
          setDescription(event.target.value);
        }}
      />
      <div className="flex gap-2">
        <Button type="submit" size="sm" disabled={title.trim() === "" || save.isPending}>
          {t("collections.save")}
        </Button>
        <Button type="button" variant="ghost" size="sm" onClick={onDone}>
          {t("collections.cancel")}
        </Button>
      </div>
      {save.isError && <ErrorMessage error={save.error} />}
    </form>
  );
}

function Items({ collectionId, items }: { collectionId: number; items: CollectionItem[] }) {
  const { t } = useTranslation();
  const move = useMoveCollectionItem();
  const ids = useMemo(() => items.map((item) => item.id), [items]);
  const sensors = useSensors(
    // A small distance, so a press on the handle does not count as a drag.
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const position = (id: string | number) => ids.indexOf(Number(id)) + 1;

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    move.mutate({ collectionId, id: Number(active.id), toIndex: ids.indexOf(Number(over.id)) });
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={onDragEnd}
      accessibility={{
        screenReaderInstructions: { draggable: t("collections.drag_instructions") },
        announcements: {
          onDragStart: ({ active }) =>
            t("collections.drag_start", { position: position(active.id) }),
          onDragOver: ({ active, over }) =>
            over
              ? t("collections.drag_over", {
                  position: position(active.id),
                  over: position(over.id),
                })
              : undefined,
          onDragEnd: ({ over }) =>
            over ? t("collections.drag_end", { over: position(over.id) }) : undefined,
          onDragCancel: () => t("collections.drag_cancel"),
        },
      }}
    >
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        <ol aria-label={t("collections.items")} className="grid gap-3">
          {items.map((item, index) => (
            <SortableItem
              key={item.id}
              item={item}
              isFirst={index === 0}
              isLast={index === items.length - 1}
              onMove={(delta) => {
                move.mutate({ collectionId, id: item.id, toIndex: index + delta });
              }}
            />
          ))}
        </ol>
      </SortableContext>
      {move.isError && <ErrorMessage error={move.error} />}
    </DndContext>
  );
}

function SortableItem({
  item,
  isFirst,
  isLast,
  onMove,
}: {
  item: CollectionItem;
  isFirst: boolean;
  isLast: boolean;
  onMove: (delta: number) => void;
}) {
  const { t } = useTranslation();
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: item.id });
  const remove = useRemoveCollectionItem();
  const name = useItemName(item);
  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      data-dragging={isDragging || undefined}
      className="flex gap-2 rounded-lg border bg-background p-3 data-dragging:z-10 data-dragging:shadow-lg"
    >
      <div className="flex shrink-0 flex-col items-center gap-0.5">
        <Button
          ref={setActivatorNodeRef}
          variant="ghost"
          size="icon-xs"
          aria-label={t("collections.drag", { item: name })}
          className="cursor-grab touch-none text-muted-foreground"
          {...attributes}
          {...listeners}
        >
          <GripVerticalIcon aria-hidden />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label={t("collections.move_up")}
          className="text-muted-foreground"
          disabled={isFirst}
          onClick={() => {
            onMove(-1);
          }}
        >
          <ArrowUpIcon aria-hidden />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label={t("collections.move_down")}
          className="text-muted-foreground"
          disabled={isLast}
          onClick={() => {
            onMove(1);
          }}
        >
          <ArrowDownIcon aria-hidden />
        </Button>
      </div>
      <div className="grid min-w-0 flex-1 gap-3">
        <CollectionItemContent item={item} />
        <Annotation key={item.annotation ?? ""} item={item} name={name} />
        {remove.isError && <ErrorMessage error={remove.error} />}
      </div>
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={t("collections.remove")}
        className="shrink-0 text-muted-foreground"
        onClick={() => {
          remove.mutate(item.id);
        }}
      >
        <XIcon aria-hidden />
      </Button>
    </li>
  );
}

/** The item's short note: saved when leaving the field. */
function Annotation({ item, name }: { item: CollectionItem; name: string }) {
  const { t } = useTranslation();
  const [text, setText] = useState(item.annotation ?? "");
  const update = useUpdateCollectionItem();
  return (
    <>
      <Input
        aria-label={`${t("collections.annotation_label")}: ${name}`}
        dir="auto"
        value={text}
        placeholder={t("collections.annotation_placeholder")}
        className="h-8 text-sm"
        onChange={(event) => {
          setText(event.target.value);
        }}
        onBlur={() => {
          const next = text.trim();
          if (next === (item.annotation ?? "")) return;
          update.mutate({ id: item.id, annotation: next === "" ? null : next });
        }}
      />
      {update.isError && <ErrorMessage error={update.error} />}
    </>
  );
}
