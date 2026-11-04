import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { CollectionSummary } from "@/lib/bindings";
import type { AddCollectionItemVariables } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { AddToCollectionDialog } from "./AddToCollectionDialog";

const summary = (id: number, title: string): CollectionSummary => ({
  id,
  title,
  description: null,
  item_count: 2,
  created_at: "2026-01-01",
  updated_at: "2026-01-01",
});

let collections: CollectionSummary[];
const addItem = vi.fn(
  (_variables: AddCollectionItemVariables, options: { onSuccess: () => void }) => {
    options.onSuccess();
  },
);
const create = vi.fn(
  (input: { title: string }, options: { onSuccess: (created: CollectionSummary) => void }) => {
    options.onSuccess(summary(9, input.title));
  },
);

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, onClick }: { children: ReactNode; onClick?: () => void }) => (
    <a href="#x" onClick={onClick}>
      {children}
    </a>
  ),
}));

vi.mock("@/lib/queries", () => ({
  useCollections: () => ({ data: collections, isError: false }),
  useSurahs: () => ({
    data: [
      { number: 1, ayah_count: 7, name_tr: "Fâtiha", name_en: "Al-Faatiha" },
      { number: 2, ayah_count: 286, name_tr: "Bakara", name_en: "Al-Baqara" },
    ],
  }),
  useAddCollectionItem: () => ({ mutate: addItem, isPending: false, isError: false }),
  useUpsertCollection: () => ({ mutate: create, isPending: false, isError: false }),
}));

describe("AddToCollectionDialog", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    collections = [summary(1, "Sabır")];
    addItem.mockClear();
    create.mockClear();
  });

  const open = () => {
    useUiStore.setState({ collectionPicker: { kind: "verse_range", from: 8, to: 8 } });
    return render(<AddToCollectionDialog />);
  };

  it("stays closed with nothing to add", () => {
    useUiStore.setState({ collectionPicker: null });
    render(<AddToCollectionDialog />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("names the verse and adds it with an annotation", () => {
    open();
    expect(screen.getByText("Adding Al-Baqara 2:1")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: /Short note/ }), {
      target: { value: " önemli " },
    });
    fireEvent.click(screen.getByRole("button", { name: /Sabır/ }));
    expect(addItem).toHaveBeenCalledWith(
      {
        collectionId: 1,
        item: { kind: "verse_range", from: 8, to: 8 },
        annotation: "önemli",
      },
      expect.anything(),
    );
    expect(screen.getByRole("status")).toHaveTextContent("Added to “Sabır”.");
  });

  it("widens a verse into a range", () => {
    open();
    fireEvent.change(screen.getByRole("spinbutton", { name: "Through verse" }), {
      target: { value: "5" },
    });
    expect(screen.getByText("Adding Al-Baqara 2:1–5")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Sabır/ }));
    expect(addItem.mock.calls[0]?.[0].item).toEqual({ kind: "verse_range", from: 8, to: 12 });
  });

  it("creates the first collection and adds to it", () => {
    collections = [];
    open();
    expect(screen.getByText(/You have no collections yet/)).toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "New collection name" }), {
      target: { value: "Yeni" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create and add" }));
    expect(create).toHaveBeenCalledOnce();
    expect(addItem.mock.calls[0]?.[0].collectionId).toBe(9);
  });

  it("closes from the link to the collection", () => {
    open();
    fireEvent.click(screen.getByRole("button", { name: /Sabır/ }));
    fireEvent.click(screen.getByText("Open the collection"));
    expect(useUiStore.getState().collectionPicker).toBeNull();
  });
});
