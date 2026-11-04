import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { VerseWithTranslations } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { VerseArabic } from "./VerseArabic";

vi.mock("./WordCard", () => ({
  WordCard: ({ wordId, onDetails }: { wordId: number; onDetails: () => void }) => (
    <button onClick={onDetails}>{`card ${String(wordId)}`}</button>
  ),
}));

const TEXT = "بِسْمِ ٱللَّهِ";
const VERSE: VerseWithTranslations = {
  id: 1,
  surah: 1,
  ayah: 1,
  juz: 1,
  text_uthmani: TEXT,
  translations: [],
  words: [
    { id: 1, position: 1, text_ar: "بِسْمِ" },
    { id: 2, position: 2, text_ar: "ٱللَّهِ" },
  ],
};

describe("VerseArabic", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    useUiStore.setState({ selectedVerse: null, selectedWord: null });
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  const renderVerse = (verse = VERSE) =>
    render(<VerseArabic verse={verse} />, { wrapper: TooltipProvider });

  it("shows the plain text without a morphology pack", () => {
    renderVerse({ ...VERSE, words: [] });
    expect(screen.getByText(TEXT)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toEqual([]);
  });

  it("renders the same text with clickable words", () => {
    const { container } = renderVerse();
    expect(container.querySelector("p")?.textContent).toBe(TEXT);
    expect(screen.getAllByRole("button")).toHaveLength(2);
  });

  it("opens the card on a click and the panel from its details", () => {
    useUiStore.getState().setPreference("word.click", "card");
    useUiStore.getState().setPreference("layout.right_panel", {
      ...DEFAULT_PREFERENCES["layout.right_panel"],
      collapsed: true,
      tab: "tafsir",
    });
    renderVerse();
    fireEvent.click(screen.getByRole("button", { name: "ٱللَّهِ" }));
    expect(useUiStore.getState().selectedWord).toBe(2);
    expect(useUiStore.getState().selectedVerse).toEqual({ surah: 1, ayah: 1 });
    expect(useUiStore.getState().preferences["layout.right_panel"].collapsed).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "card 2" }));
    const panel = useUiStore.getState().preferences["layout.right_panel"];
    expect(panel.collapsed).toBe(false);
    expect(panel.tab).toBe("word");
  });

  it("opens the panel directly in the panel mode", () => {
    useUiStore.getState().setPreference("word.click", "panel");
    useUiStore.getState().setPreference("layout.right_panel", {
      ...DEFAULT_PREFERENCES["layout.right_panel"],
      collapsed: true,
      tab: "tafsir",
    });
    renderVerse();
    fireEvent.click(screen.getByRole("button", { name: "بِسْمِ" }));
    expect(screen.queryByRole("button", { name: /card/ })).not.toBeInTheDocument();
    const panel = useUiStore.getState().preferences["layout.right_panel"];
    expect(panel.collapsed).toBe(false);
    expect(panel.tab).toBe("word");
  });

  it("a click ending a text selection selects no word", () => {
    const selection = vi
      .spyOn(window, "getSelection")
      .mockReturnValue({ toString: () => "بِسْمِ" } as Selection);
    renderVerse();
    fireEvent.click(screen.getByRole("button", { name: "بِسْمِ" }));
    expect(useUiStore.getState().selectedWord).toBeNull();
    selection.mockRestore();
  });

  it("marks the selected word only in its verse", () => {
    useUiStore.getState().selectWord({ surah: 1, ayah: 1 }, 2);
    renderVerse();
    expect(screen.getByRole("button", { name: "ٱللَّهِ" })).toHaveAttribute("data-selected");
    expect(screen.getByRole("button", { name: "بِسْمِ" })).not.toHaveAttribute("data-selected");
  });
});
