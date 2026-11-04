import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { MorphologyPack, WordDetail } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { WordPanel } from "./WordPanel";

let pack: MorphologyPack | null;
const openSource = vi.fn();

const WORD: WordDetail = {
  id: 2,
  surah: 1,
  ayah: 1,
  position: 2,
  text_ar: "بِسْمِ",
  translit: null,
  gloss_en: null,
  segments: [
    { kind: "prefix", text_ar: "بِ", pos: "P", root: null, lemma: null, features: [] },
    {
      kind: "stem",
      text_ar: "سْمِ",
      pos: "N",
      root: { id: 5, root: "س م و", occurrences: 381 },
      lemma: { id: 6, lemma: "ٱسْم", occurrences: 39 },
      features: [{ key: "case", value: "GEN" }],
    },
  ],
};

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    to,
    params,
  }: {
    children: ReactNode;
    to: string;
    params: Record<string, string | number>;
  }) => (
    <a
      href={Object.entries(params).reduce(
        (path, [key, value]) => path.replace(`$${key}`, String(value)),
        to,
      )}
    >
      {children}
    </a>
  ),
}));

vi.mock("@/lib/queries", () => ({
  useMorphologyPack: () => ({ data: pack, isPending: false, isError: false }),
  useSurahs: () => ({ data: [{ number: 1, ayah_count: 7 }] }),
  useVerseWords: () => ({
    data: [
      { id: 1, position: 1, text_ar: "ٱللَّهِ" },
      { id: 2, position: 2, text_ar: "بِسْمِ" },
    ],
    isError: false,
  }),
  useWord: () => ({ data: WORD, isError: false }),
  useOpenPackSource: () => ({ mutate: openSource }),
  useCatalog: () => ({ data: undefined }),
}));

describe("WordPanel", () => {
  beforeEach(async () => {
    pack = { id: "morphology.qac", source: "Quranic Arabic Corpus (corpus.quran.com)" };
    openSource.mockClear();
    await i18n.changeLanguage("en");
    useUiStore.setState({ selectedVerse: null, selectedWord: null });
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  const renderPanel = (verse = { surah: 1, ayah: 1 } as { surah: number; ayah: number } | null) =>
    render(<WordPanel verse={verse} />, { wrapper: TooltipProvider });

  it("asks for the pack without one", () => {
    pack = null;
    renderPanel();
    expect(screen.getByRole("button", { name: "Open Packs" })).toBeInTheDocument();
  });

  it("asks to select something when no verse is selected", () => {
    renderPanel(null);
    expect(screen.getByText(/Click a word in the reader/)).toBeInTheDocument();
  });

  it("lists the verse's words, and choosing one shows its analysis", () => {
    renderPanel();
    expect(screen.queryByRole("region", { name: "Word analysis" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "بِسْمِ" }));
    expect(useUiStore.getState().selectedWord).toBe(2);
    expect(screen.getByRole("region", { name: "Word analysis" })).toBeInTheDocument();
    expect(screen.getByText("Preposition")).toBeInTheDocument();
    expect(screen.getByText("Noun")).toBeInTheDocument();
    expect(screen.getByText("Genitive")).toBeInTheDocument();
    expect(screen.getByText("س م و")).toBeInTheDocument();
  });

  it("links to the concordance with the counts", () => {
    useUiStore.getState().selectWord({ surah: 1, ayah: 1 }, 2);
    renderPanel();
    expect(screen.getByRole("link", { name: "Where this root occurs (381)" })).toHaveAttribute(
      "href",
      "/concordance/root/5",
    );
    expect(
      screen.getByRole("link", { name: "Where this dictionary form occurs (39)" }),
    ).toHaveAttribute("href", "/concordance/lemma/6");
  });

  it("names the source and opens its link", () => {
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Quranic Arabic Corpus/ }));
    expect(openSource).toHaveBeenCalledWith("morphology.qac");
  });

  it("ignores a word kept from another verse", () => {
    useUiStore.getState().selectWord({ surah: 1, ayah: 1 }, 99);
    renderPanel();
    expect(screen.queryByRole("region", { name: "Word analysis" })).not.toBeInTheDocument();
  });
});
