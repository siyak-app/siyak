import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { TafsirInfo, TafsirPassage } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { TafsirPanel } from "./TafsirPanel";

const JALALAYN: TafsirInfo = {
  id: "ar.jalalayn",
  title: "Jalalayn",
  titles: { tr: "Celâleyn", en: "Jalalayn" },
  author: "al-Mahalli",
  lang: "ar",
  dir: "rtl",
  attribution: "Source: Quranpedia",
  has_introduction: true,
};

const RANGE: TafsirPassage = {
  id: 12,
  verse_from: { surah: 2, ayah: 153 },
  verse_to: { surah: 2, ayah: 157 },
  paragraphs: [
    { idx: 0, text: "first", part: "1", page: 113 },
    { idx: 1, text: "second", part: "1", page: 113 },
    { idx: 2, text: "third", part: "1", page: 114 },
  ],
};

const INTRODUCTION: TafsirPassage = {
  id: 1,
  verse_from: { surah: 1, ayah: 1 },
  verse_to: { surah: 1, ayah: 1 },
  paragraphs: [{ idx: 0, text: "muqaddima", part: null, page: null }],
};

let tafsirs: TafsirInfo[];
let passage: TafsirPassage | null;
const requested: [number, string][] = [];

vi.mock("@/features/ai/LlmPicker", () => ({
  LlmPicker: ({ role }: { role: string }) => <p>{role} model picker</p>,
}));

vi.mock("@/lib/queries", () => ({
  useTafsirs: () => ({ data: tafsirs, isPending: false, isError: false }),
  useSurahs: () => ({
    data: [
      { number: 1, ayah_count: 7 },
      { number: 2, ayah_count: 286 },
    ],
    isError: false,
  }),
  useTafsir: (verseId: number, packId: string) => {
    requested.push([verseId, packId]);
    return { data: passage, isPending: false, isError: false };
  },
  useTafsirIntroduction: () => ({ data: INTRODUCTION, isPending: false, isError: false }),
  useAiStatus: () => ({ data: { state: "no_provider" } }),
  useTafsirTranslations: () => ({ data: [], isError: false }),
  useTafsirEstimate: () => ({ data: undefined }),
  useTafsirTranslated: () => () => Promise.resolve(),
  translateTafsir: () => Promise.reject(new Error("not in this test")),
  cancelGeneration: () => Promise.resolve(null),
  appErrorCode: () => undefined,
}));

const VERSE = { surah: 2, ayah: 155 };

describe("TafsirPanel", () => {
  it("keeps the tafsir in use when another one is installed", () => {
    const view = render(<TafsirPanel verse={VERSE} />);
    expect(useUiStore.getState().preferences["tafsir.active"]).toBe("ar.jalalayn");
    tafsirs = [
      { ...JALALAYN, id: "ar.earlier", title: "Earlier", titles: { tr: null, en: null } },
      JALALAYN,
    ];
    view.rerender(<TafsirPanel verse={VERSE} />);
    expect(requested.at(-1)?.[1]).toBe("ar.jalalayn");
  });

  beforeEach(async () => {
    tafsirs = [JALALAYN];
    passage = RANGE;
    requested.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
    useUiStore.getState().setPacksDialogOpen(false);
  });

  it("offers the Packs screen when no tafsir is installed", () => {
    tafsirs = [];
    render(<TafsirPanel verse={VERSE} />);
    fireEvent.click(screen.getByRole("button", { name: "Open Packs" }));
    expect(useUiStore.getState().packsDialogOpen).toBe(true);
    expect(requested).toEqual([]);
  });

  it("asks for a verse until one is selected", () => {
    render(<TafsirPanel verse={null} />);
    expect(screen.getByText("Select a verse to read its commentary.")).toBeInTheDocument();
    expect(requested).toEqual([]);
  });

  it("shows the covering range, text direction, pages and attribution", () => {
    render(<TafsirPanel verse={VERSE} />);
    expect(requested[0]).toEqual([162, "ar.jalalayn"]);
    expect(screen.getByText("This commentary covers verses 2:153–157.")).toBeInTheDocument();
    const first = screen.getByText("first");
    expect(first.closest("[dir]")).toHaveAttribute("dir", "rtl");
    expect(first.closest("[lang]")).toHaveAttribute("lang", "ar");
    // A page is marked once, where it starts.
    expect(screen.getByText("first")).toHaveTextContent("[1/113]");
    expect(screen.getByText("second")).not.toHaveTextContent("[");
    expect(screen.getByText("third")).toHaveTextContent("[1/114]");
    expect(screen.getByText(/Source: Quranpedia/)).toBeInTheDocument();
  });

  it("says so when the tafsir has no commentary for the verse", () => {
    passage = null;
    render(<TafsirPanel verse={VERSE} />);
    expect(
      screen.getByText("This tafsir has no separate commentary for this verse."),
    ).toBeInTheDocument();
  });

  it("does not mention a range for a single verse", () => {
    passage = { ...RANGE, verse_to: RANGE.verse_from };
    render(<TafsirPanel verse={VERSE} />);
    expect(screen.queryByText(/covers verses/)).not.toBeInTheDocument();
  });

  it("opens the introduction separately and returns on a new verse", () => {
    const { rerender } = render(<TafsirPanel verse={VERSE} />);
    fireEvent.click(screen.getByRole("button", { name: "Author's introduction" }));
    expect(screen.getByText("muqaddima")).toBeInTheDocument();
    expect(screen.queryByText("first")).not.toBeInTheDocument();
    rerender(<TafsirPanel verse={{ surah: 2, ayah: 156 }} />);
    expect(screen.getByText("first")).toBeInTheDocument();
    expect(screen.queryByText("muqaddima")).not.toBeInTheDocument();
  });

  it("falls back to an installed tafsir when the stored one was removed", () => {
    useUiStore.getState().setPreference("tafsir.active", "ar.removed");
    render(<TafsirPanel verse={VERSE} />);
    expect(requested[0]).toEqual([162, "ar.jalalayn"]);
  });
});
