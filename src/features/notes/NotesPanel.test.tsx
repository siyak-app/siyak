import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { Note, NoteInput } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { NotesPanel } from "./NotesPanel";

const NOTE: Note = {
  id: 4,
  verse_from: 3,
  verse_to: 5,
  from: { surah: 1, ayah: 3 },
  to: { surah: 1, ayah: 5 },
  body: "Benim **notum**",
  created_at: "2026-01-01",
  updated_at: "2026-01-01",
};

let notes: Note[];
const ranges: unknown[] = [];
const upsert = vi.fn((_note: NoteInput, options: { onSuccess: () => void }) => {
  options.onSuccess();
});
const remove = vi.fn();

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children }: { children: ReactNode }) => <a href="#x">{children}</a>,
}));

vi.mock("@/lib/queries", () => ({
  useSurahs: () => ({
    data: [{ number: 1, ayah_count: 7, name_tr: "Fâtiha", name_en: "Al-Faatiha" }],
    isError: false,
  }),
  useNotes: (range: unknown) => {
    ranges.push(range);
    return { data: notes, isError: false };
  },
  useUpsertNote: () => ({ mutate: upsert, isPending: false, isError: false }),
  useDeleteNote: () => ({ mutate: remove, isError: false }),
}));

function show(verse: { surah: number; ayah: number } | null = { surah: 1, ayah: 3 }) {
  return render(<NotesPanel verse={verse} />, { wrapper: TooltipProvider });
}

describe("NotesPanel", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    notes = [];
    ranges.length = 0;
    upsert.mockClear();
    remove.mockClear();
    useUiStore.setState({ preferences: DEFAULT_PREFERENCES, noteFocusPending: false });
  });

  it("asks for a verse when none is selected", () => {
    show(null);
    expect(screen.getByText("Select a verse to see and write its notes.")).toBeInTheDocument();
  });

  it("lists the notes touching the verse", () => {
    notes = [NOTE];
    show();
    expect(ranges).toContainEqual({ from: 3, to: 3 });
    expect(screen.getByText("notum").tagName).toBe("STRONG");
    expect(screen.getByRole("article", { name: "Al-Faatiha 1:3–5" })).toBeInTheDocument();
  });

  it("saves a new note over a range of verses", () => {
    show();
    const save = screen.getByRole("button", { name: "Save note" });
    expect(save).toBeDisabled();
    const body = screen.getByRole("textbox", { name: "New note" });
    fireEvent.change(body, { target: { value: "Yeni" } });
    fireEvent.change(screen.getByRole("spinbutton", { name: "Through verse" }), {
      target: { value: "5" },
    });
    fireEvent.click(save);
    expect(upsert).toHaveBeenCalledWith(
      { id: null, verse_from: 3, verse_to: 5, body: "Yeni" },
      expect.anything(),
    );
    expect(body).toHaveValue("");
  });

  it("saves with Ctrl+Enter", () => {
    show();
    const body = screen.getByRole("textbox", { name: "New note" });
    fireEvent.change(body, { target: { value: "Yeni" } });
    fireEvent.keyDown(body, { key: "Enter", ctrlKey: true });
    expect(upsert).toHaveBeenCalledOnce();
  });

  it("does not end a range before it starts", () => {
    show();
    fireEvent.change(screen.getByRole("textbox", { name: "New note" }), {
      target: { value: "Yeni" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "Through verse" }), {
      target: { value: "1" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    expect(upsert.mock.calls[0]?.[0]).toMatchObject({ verse_from: 3, verse_to: 3 });
  });

  it("changes a note in place", () => {
    notes = [NOTE];
    show();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const body = screen.getByRole("textbox", { name: "Note text" });
    expect(body).toHaveValue(NOTE.body);
    fireEvent.change(body, { target: { value: "Değişti" } });
    fireEvent.keyDown(body, { key: "Enter", metaKey: true });
    expect(upsert).toHaveBeenCalledWith(
      { id: 4, verse_from: 3, verse_to: 5, body: "Değişti" },
      expect.anything(),
    );
  });

  it("deletes a note only after confirming", () => {
    notes = [NOTE];
    show();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(remove).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete" }));
    expect(remove).toHaveBeenCalledWith(4);
  });

  it("focuses the editor when the n key asked for it", () => {
    useUiStore.setState({ noteFocusPending: true });
    show();
    expect(screen.getByRole("textbox", { name: "New note" })).toHaveFocus();
    expect(useUiStore.getState().noteFocusPending).toBe(false);
  });
});
