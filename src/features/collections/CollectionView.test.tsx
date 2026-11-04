import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { Collection, CollectionItem } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { CollectionView } from "./CollectionView";

const VERSE = {
  id: 8,
  surah: 2,
  ayah: 1,
  juz: 1,
  text_uthmani: "الٓمٓ",
  words: [],
  translations: [{ pack_id: "tr.diyanet", lang: "tr", text: "Elif Lâm Mîm" }],
};

const ITEMS: CollectionItem[] = [
  {
    id: 10,
    item: { kind: "verse_range", from: 8, to: 8 },
    annotation: "Başlangıç",
    content: { kind: "verse_range", verses: [VERSE] },
  },
  {
    id: 11,
    item: { kind: "note", id: 3 },
    annotation: null,
    content: { kind: "note", note: null },
  },
  {
    id: 12,
    item: { kind: "root", root: "ص ب ر" },
    annotation: null,
    content: { kind: "root", root: { id: 7, root: "ص ب ر", occurrences: 103 } },
  },
  {
    id: 13,
    item: { kind: "tafsir", pack_id: "gone", verse: 8 },
    annotation: null,
    content: { kind: "tafsir", verse: { surah: 2, ayah: 1 }, tafsir: null, passage: null },
  },
];

const collection: Collection = {
  summary: {
    id: 1,
    title: "Sabır",
    description: "Konu **notu**",
    item_count: 4,
    created_at: "2026-01-01",
    updated_at: "2026-01-01",
  },
  items: ITEMS,
};

let current: Collection;
const move = vi.fn();
const remove = vi.fn();
const update = vi.fn();
const exportMarkdown = vi.fn();
const deleteCollection = vi.fn();
const save = vi.fn();
const translationIds: unknown[] = [];

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
  Link: ({ children, className }: { children: ReactNode; className?: string }) => (
    <a href="#x" className={className}>
      {children}
    </a>
  ),
}));

vi.mock("@/lib/queries", () => ({
  appErrorCode: () => undefined,
  useCollection: (_id: number, ids: unknown) => {
    translationIds.push(ids);
    return { data: current, isError: false, isPending: false };
  },
  useSurahs: () => ({
    data: [
      { number: 1, ayah_count: 7, name_tr: "Fâtiha", name_en: "Al-Faatiha" },
      { number: 2, ayah_count: 286, name_tr: "Bakara", name_en: "Al-Baqara" },
    ],
  }),
  useTranslations: () => ({ data: [] }),
  useMorphologyPack: () => ({ data: null }),
  useCatalog: () => ({ data: undefined }),
  useOpenPackSource: () => ({ mutate: vi.fn() }),
  useMoveCollectionItem: () => ({ mutate: move, isError: false }),
  useRemoveCollectionItem: () => ({ mutate: remove, isError: false }),
  useUpdateCollectionItem: () => ({ mutate: update, isError: false }),
  useDeleteCollection: () => ({ mutate: deleteCollection, isError: false }),
  useUpsertCollection: () => ({ mutate: save, isPending: false, isError: false }),
  useExportCollectionMarkdown: () => ({
    mutate: exportMarkdown,
    isPending: false,
    isError: false,
    data: null,
  }),
}));

function show() {
  return render(<CollectionView id={1} />, { wrapper: TooltipProvider });
}

function itemAt(index: number) {
  const list = screen.getByRole("list", { name: "Items of the collection" });
  const item = within(list).getAllByRole("listitem")[index];
  if (!item) throw new Error(`no item ${String(index)}`);
  return within(item);
}

describe("CollectionView", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    current = collection;
    for (const mock of [move, remove, update, exportMarkdown, deleteCollection, save]) {
      mock.mockClear();
    }
    useUiStore.setState({ preferences: DEFAULT_PREFERENCES });
  });

  it("shows the title, the description and every kind of item", () => {
    show();
    expect(screen.getByRole("heading", { name: "Sabır" })).toBeInTheDocument();
    expect(screen.getByText("notu").tagName).toBe("STRONG");
    expect(itemAt(0).getByText("Elif Lâm Mîm")).toBeInTheDocument();
    expect(itemAt(2).getByText("ص ب ر")).toBeInTheDocument();
  });

  it("says what is gone instead of hiding the item", () => {
    show();
    expect(itemAt(1).getByText("This note has been deleted.")).toBeInTheDocument();
    expect(itemAt(3).getByText(/not available/)).toBeInTheDocument();
  });

  it("moves an item with its buttons, within the ends", () => {
    show();
    expect(itemAt(0).getByRole("button", { name: "Move up" })).toBeDisabled();
    expect(itemAt(3).getByRole("button", { name: "Move down" })).toBeDisabled();
    fireEvent.click(itemAt(0).getByRole("button", { name: "Move down" }));
    expect(move).toHaveBeenCalledWith({ collectionId: 1, id: 10, toIndex: 1 });
    fireEvent.click(itemAt(3).getByRole("button", { name: "Move up" }));
    expect(move).toHaveBeenCalledWith({ collectionId: 1, id: 13, toIndex: 2 });
  });

  it("offers a named drag handle for every item", () => {
    show();
    expect(itemAt(0).getByRole("button", { name: "Reorder Al-Baqara 2:1" })).toBeInTheDocument();
  });

  it("saves an annotation on leaving the field, and only a change", () => {
    show();
    const field = itemAt(0).getByRole("textbox", { name: /Short note/ });
    expect(field).toHaveValue("Başlangıç");
    fireEvent.blur(field);
    expect(update).not.toHaveBeenCalled();
    fireEvent.change(field, { target: { value: "  Yeni " } });
    fireEvent.blur(field);
    expect(update).toHaveBeenCalledWith({ id: 10, annotation: "Yeni" });
    fireEvent.change(field, { target: { value: "" } });
    fireEvent.blur(field);
    expect(update).toHaveBeenLastCalledWith({ id: 10, annotation: null });
  });

  it("removes an item", () => {
    show();
    fireEvent.click(itemAt(1).getByRole("button", { name: "Remove from the collection" }));
    expect(remove).toHaveBeenCalledWith(11);
  });

  it("exports with the words of the interface and the chosen style", () => {
    useUiStore.getState().setPreference("reader.citation_style", "apa");
    show();
    fireEvent.click(screen.getByRole("button", { name: "Export as Markdown…" }));
    expect(exportMarkdown).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 1,
        style: "apa",
        labels: expect.objectContaining({
          language: "en",
          bibliography: "Bibliography",
        }) as unknown,
      }),
    );
  });

  it("deletes only after confirming", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Delete collection" }));
    expect(deleteCollection).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));
    expect(deleteCollection).toHaveBeenCalledWith(1, expect.anything());
  });

  it("explains an empty collection", () => {
    current = { ...collection, items: [] };
    show();
    expect(screen.getByText(/Nothing here yet/)).toBeInTheDocument();
  });

  it("renames", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Edit name and description" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), {
      target: { value: "Tevekkül" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(save).toHaveBeenCalledWith(
      { id: 1, title: "Tevekkül", description: "Konu **notu**" },
      expect.anything(),
    );
  });
});
