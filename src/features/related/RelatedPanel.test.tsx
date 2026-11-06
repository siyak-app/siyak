import { render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { MorphologyPack, RelatedVerse, SemanticStatus } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { RelatedPanel } from "./RelatedPanel";

let semanticStatus: SemanticStatus;
let morphologyPack: MorphologyPack | null;
let byMeaning: RelatedVerse[];
let byRoots: RelatedVerse[];
const calls: { kind: string; verseId: number | null; translationId: string | null }[] = [];

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    to,
    params,
    className,
  }: {
    children: ReactNode;
    to: string;
    params?: Record<string, number | string>;
    className?: string;
  }) => (
    <a
      className={className}
      href={Object.entries(params ?? {}).reduce(
        (path, [key, value]) => path.replace(`$${key}`, String(value)),
        to,
      )}
    >
      {children}
    </a>
  ),
}));

vi.mock("@/lib/queries", () => ({
  useSurahs: () => ({
    data: [
      { number: 1, name_tr: "Fâtiha", name_en: "Al-Fatiha", name_ar: "الفاتحة", ayah_count: 7 },
      { number: 2, name_tr: "Bakara", name_en: "Al-Baqara", name_ar: "البقرة", ayah_count: 286 },
    ],
  }),
  useSemanticStatus: () => ({ data: semanticStatus, isError: false }),
  useMorphologyPack: () => ({ data: morphologyPack, isError: false, isPending: false }),
  useRelatedByMeaning: (verseId: number | null, translationId: string | null, enabled: boolean) => {
    if (enabled) calls.push({ kind: "meaning", verseId, translationId });
    return { data: enabled ? byMeaning : undefined, isError: false, error: null };
  },
  useRelatedByRoots: (verseId: number | null, translationId: string | null, enabled: boolean) => {
    if (enabled) calls.push({ kind: "roots", verseId, translationId });
    return { data: enabled ? byRoots : undefined, isError: false, error: null };
  },
  useCatalog: () => ({ data: undefined }),
  useOpenPackSource: () => ({ mutate: vi.fn() }),
  appErrorCode: () => undefined,
}));

const MEANING: RelatedVerse = {
  surah: 2,
  ayah: 153,
  text_ar: "يَـٰٓأَيُّهَا ٱلَّذِينَ ءَامَنُوا۟ ٱسْتَعِينُوا۟ بِٱلصَّبْرِ",
  translation: { pack_id: "en.pickthall", lang: "en", text: "O ye who believe! Seek help" },
  shared: [],
};

const ROOTS: RelatedVerse = {
  surah: 1,
  ayah: 3,
  text_ar: "ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
  translation: null,
  shared: [
    { target: { kind: "root", id: 2 }, text: "ر ح م" },
    { target: { kind: "lemma", id: 5 }, text: "مُوسَىٰ" },
  ],
};

function section(name: string) {
  const found = screen.getByRole("heading", { name }).closest("section");
  if (!found) throw new Error(`no section ${name}`);
  return found;
}

describe("RelatedPanel", () => {
  beforeEach(async () => {
    calls.length = 0;
    semanticStatus = { state: "ready", pack: "vectors.bge-m3", provider: null };
    morphologyPack = { id: "morphology.qac", source: "Quranic Arabic Corpus" };
    byMeaning = [MEANING];
    byRoots = [ROOTS];
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate({
      ...DEFAULT_PREFERENCES,
      "reader.translations": ["en.pickthall"],
    });
  });

  it("asks for a verse first", () => {
    render(<RelatedPanel verse={null} />, { wrapper: TooltipProvider });
    expect(screen.getByText("Select a verse to see related verses.")).toBeInTheDocument();
    expect(calls).toHaveLength(0);
  });

  it("shows verses by meaning and by shared roots, with their reasons", () => {
    render(<RelatedPanel verse={{ surah: 2, ayah: 1 }} />, { wrapper: TooltipProvider });
    // 2:1 is verse 8: al-Fatiha has 7.
    expect(calls).toEqual([
      { kind: "meaning", verseId: 8, translationId: "en.pickthall" },
      { kind: "roots", verseId: 8, translationId: "en.pickthall" },
    ]);
    const meaning = section("Similar in meaning");
    const link = within(meaning).getByRole("link", { name: /Seek help/ });
    expect(link).toHaveAttribute("href", "/read/2/153");
    // The side panel shows the translation alone when there is one.
    expect(within(meaning).queryByText(MEANING.text_ar)).not.toBeInTheDocument();

    const roots = section("Shared roots");
    expect(within(roots).getByText(ROOTS.text_ar)).toBeInTheDocument();
    expect(within(roots).getByText("Shared:")).toBeInTheDocument();
    expect(within(roots).getByRole("link", { name: "ر ح م" })).toHaveAttribute(
      "href",
      "/concordance/root/2",
    );
    expect(within(roots).getByRole("link", { name: /مُوسَىٰ/ })).toHaveAttribute(
      "href",
      "/concordance/lemma/5",
    );
    expect(within(roots).getByText("Quranic Arabic Corpus")).toBeInTheDocument();
  });

  it("compares verse vectors even when the model pack is missing", () => {
    semanticStatus = {
      state: "model_pack_missing",
      pack: "vectors.bge-m3",
      model_pack: "model.bge-m3",
    };
    render(<RelatedPanel verse={{ surah: 1, ayah: 1 }} />, { wrapper: TooltipProvider });
    expect(calls.map((call) => call.kind)).toContain("meaning");
    expect(within(section("Similar in meaning")).getByRole("link")).toBeInTheDocument();
  });

  it("says which pack each section needs", () => {
    semanticStatus = { state: "no_vector_pack" };
    morphologyPack = null;
    render(<RelatedPanel verse={{ surah: 1, ayah: 1 }} />, { wrapper: TooltipProvider });
    expect(calls).toHaveLength(0);
    expect(
      within(section("Similar in meaning")).getByText(
        "Searching by meaning needs the semantic search packs.",
      ),
    ).toBeInTheDocument();
    expect(
      within(section("Shared roots")).getByText("This needs the word analysis pack."),
    ).toBeInTheDocument();
  });

  it("says when nothing is related", () => {
    byMeaning = [];
    byRoots = [];
    render(<RelatedPanel verse={{ surah: 1, ayah: 1 }} />, { wrapper: TooltipProvider });
    expect(screen.getAllByText("Nothing found.")).toHaveLength(2);
  });
});
