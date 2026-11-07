import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type {
  MorphologyPack,
  QueryClass,
  RelatedVerse,
  RootSearch,
  SearchHit,
  SearchResults,
  SemanticStatus,
} from "@/lib/bindings";
import type { MeaningVariables, SearchTextVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { SearchView } from "./SearchView";

const navigate = vi.fn();
let classified: QueryClass;
let results: SearchResults;
const searches: SearchTextVariables[] = [];
let morphologyPack: MorphologyPack | null;
let rootSearch: RootSearch;
let semanticStatus: SemanticStatus;
let meaning: RelatedVerse[];
const meaningSearches: MeaningVariables[] = [];
const ROOT = { id: 7, root: "ص ب ر", occurrences: 103 };
const LEMMA = { id: 9, lemma: "صَبْر", occurrences: 20 };

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigate,
  Link: ({
    children,
    to,
    params,
    search,
    className,
  }: {
    children: ReactNode;
    to: string;
    params?: Record<string, number>;
    search?: Record<string, string>;
    className?: string;
  }) => (
    <a
      className={className}
      href={`${Object.entries(params ?? {}).reduce(
        (path, [key, value]) => path.replace(`$${key}`, String(value)),
        to,
      )}${search ? `?${new URLSearchParams(search).toString()}` : ""}`}
    >
      {children}
    </a>
  ),
}));

vi.mock("@/features/ai/EmbeddingPicker", () => ({
  EmbeddingPicker: () => <p>search model picker</p>,
}));

vi.mock("@/lib/queries", () => ({
  useMorphologyPack: () => ({ data: morphologyPack }),
  useSearchRoots: () => ({ data: rootSearch, isError: false }),
  useOpenPackSource: () => ({ mutate: vi.fn() }),
  useCatalog: () => ({ data: undefined }),
  useConcordance: () => ({
    data: {
      root: rootSearch.roots[0] ?? null,
      lemma: null,
      lemmas: rootSearch.lemmas,
      occurrences: 103,
      verse_count: 90,
      by_surah: [],
      verses: [],
    },
    isError: false,
  }),
  useClassifyQuery: () => ({ data: classified, isError: false }),
  useSemanticStatus: () => ({ data: semanticStatus, isError: false }),
  useSearchMeaning: (variables: MeaningVariables, enabled: boolean) => {
    if (enabled) meaningSearches.push(variables);
    return { data: enabled ? meaning : undefined, isError: false };
  },
  useSearchText: (variables: SearchTextVariables) => {
    searches.push(variables);
    return { data: results, isError: false };
  },
  useSurahs: () => ({
    data: [{ number: 2, name_tr: "Bakara", name_en: "Al-Baqara", name_ar: "البقرة" }],
  }),
  useTranslations: () => ({
    data: [
      {
        id: "en.pickthall",
        title: "Pickthall",
        titles: { tr: null, en: null },
        lang: "en",
        dir: "ltr",
        bundled: true,
      },
      {
        id: "tr.test",
        title: "Deneme",
        titles: { tr: null, en: null },
        lang: "tr",
        dir: "ltr",
        bundled: false,
      },
    ],
  }),
  appErrorCode: () => undefined,
}));

const TRANSLATION: SearchHit = {
  surah: 2,
  ayah: 153,
  pack_id: "en.pickthall",
  parts: [
    { text: "seek help in ", matched: false },
    { text: "patience", matched: true },
  ],
};

const SIMILAR: RelatedVerse = {
  surah: 2,
  ayah: 45,
  text_ar: "وَٱسْتَعِينُوا۟ بِٱلصَّبْرِ",
  translation: { pack_id: "en.pickthall", lang: "en", text: "Seek help in patience and prayer" },
  shared: [],
};

const ARABIC: SearchHit = {
  surah: 2,
  ayah: 153,
  pack_id: null,
  parts: [
    { text: "مع ", matched: false },
    { text: "الصابرين", matched: true },
  ],
};

describe("SearchView", () => {
  beforeEach(async () => {
    navigate.mockClear();
    searches.length = 0;
    classified = { kind: "text" };
    results = { total: 1, hits: [TRANSLATION] };
    morphologyPack = { id: "morphology.qac", source: "Quranic Arabic Corpus" };
    rootSearch = { roots: [], lemmas: [] };
    semanticStatus = { state: "no_vector_pack" };
    meaning = [SIMILAR];
    meaningSearches.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("searches translations in All mode and links to the verse", () => {
    render(<SearchView q="patience" mode="all" />, { wrapper: TooltipProvider });
    expect(searches[0]).toMatchObject({ scope: "translations", limit: 5, packIds: [] });
    const link = screen.getByRole("link");
    expect(link).toHaveAttribute("href", "/read/2/153");
    expect(link).toHaveTextContent("Al-Baqara 2:153 · Pickthall");
    expect(within(link).getByText("patience").tagName).toBe("MARK");
    expect(within(link).getByText("seek help in").tagName).toBe("SPAN");
  });

  it("searches the Arabic text for Arabic queries, right to left", () => {
    classified = { kind: "arabic" };
    results = { total: 1, hits: [ARABIC] };
    render(<SearchView q="الصابرين" mode="all" />, { wrapper: TooltipProvider });
    expect(searches[0]?.scope).toBe("arabic");
    const mark = screen.getByText("الصابرين", { selector: "mark" });
    expect(mark.parentElement).toHaveAttribute("dir", "rtl");
    expect(mark.parentElement).toHaveAttribute("lang", "ar");
  });

  it("offers the full list when All shows only the first results", () => {
    results = { total: 40, hits: [TRANSLATION] };
    render(<SearchView q="patience" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("link", { name: "See all 40 results" })).toHaveAttribute(
      "href",
      "/search?q=patience&mode=text",
    );
  });

  it("filters and lengthens the list in Text mode", () => {
    results = { total: 60, hits: [TRANSLATION] };
    render(<SearchView q="patience" mode="text" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("status")).toHaveTextContent("Results: 60");
    expect(searches.at(-1)).toMatchObject({ limit: 50 });

    fireEvent.click(screen.getByRole("button", { name: "Show more" }));
    expect(searches.at(-1)).toMatchObject({ limit: 100 });

    fireEvent.click(screen.getByRole("radio", { name: "Medina" }));
    expect(searches.at(-1)).toMatchObject({
      limit: 50,
      filters: { revelation: "medinan" },
    });

    fireEvent.change(screen.getByLabelText("From"), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("To"), { target: { value: "" } });
    expect(searches.at(-1)).toMatchObject({
      filters: { surah_from: 2, surah_to: null, revelation: "medinan" },
    });

    fireEvent.click(screen.getByRole("checkbox", { name: "Deneme" }));
    expect(searches.at(-1)).toMatchObject({ packIds: ["tr.test"] });
  });

  it("has no translation filter for the Arabic text", () => {
    classified = { kind: "arabic" };
    results = { total: 1, hits: [ARABIC] };
    render(<SearchView q="الصابرين" mode="text" />, { wrapper: TooltipProvider });
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("says when nothing was found", () => {
    results = { total: 0, hits: [] };
    render(<SearchView q="zzz" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByText(/Nothing found/)).toBeInTheDocument();
  });

  it("does not search for references, bad references, roots or nothing", () => {
    classified = { kind: "reference", surah: 2, ayah: 255, ayah_to: null };
    const { unmount } = render(<SearchView q="2:255" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("link", { name: "Go to 2:255" })).toHaveAttribute(
      "href",
      "/read/2/255",
    );
    unmount();

    classified = { kind: "invalid_reference", reason: "ayah_out_of_range" };
    const second = render(<SearchView q="2:999" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByText("That surah has no such verse.")).toBeInTheDocument();
    second.unmount();

    classified = { kind: "latin_root" };
    const third = render(<SearchView q="x-y-z" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByText(/No root or dictionary form matches/)).toBeInTheDocument();
    third.unmount();

    classified = { kind: "empty" };
    render(<SearchView q="" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByText(/Type a word/)).toBeInTheDocument();
    expect(searches).toEqual([]);
  });

  it("shows mode chips only when the setting is on", () => {
    const { unmount } = render(<SearchView q="patience" mode="all" />, {
      wrapper: TooltipProvider,
    });
    expect(screen.queryByRole("group", { name: "Search mode" })).not.toBeInTheDocument();
    unmount();

    useUiStore.getState().setPreference("search.mode_chips", true);
    render(<SearchView q="patience" mode="all" />, { wrapper: TooltipProvider });
    fireEvent.click(screen.getByRole("button", { name: "Text" }));
    expect(navigate).toHaveBeenCalledWith({
      to: "/search",
      search: { q: "patience", mode: "text" },
    });
  });

  it("puts the root and dictionary form above the Arabic matches", () => {
    classified = { kind: "arabic" };
    rootSearch = { roots: [ROOT], lemmas: [LEMMA] };
    render(<SearchView q="صبر" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("link", { name: /ص ب ر/ })).toHaveAttribute(
      "href",
      "/concordance/root/7",
    );
    expect(screen.getByRole("link", { name: /صَبْر/ })).toHaveAttribute(
      "href",
      "/concordance/lemma/9",
    );
    const headings = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent);
    expect(headings).toEqual(["Roots and dictionary forms", "In the Arabic text"]);
  });

  it("shows the pack card instead of the root group without the pack", () => {
    classified = { kind: "arabic" };
    morphologyPack = null;
    render(<SearchView q="صبر" mode="all" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("button", { name: "Open Packs" })).toBeInTheDocument();
    expect(screen.getByText("In the Arabic text")).toBeInTheDocument();
  });

  it("opens the concordance in root mode when the query means one root", () => {
    classified = { kind: "latin_root" };
    rootSearch = { roots: [ROOT], lemmas: [] };
    render(<SearchView q="s-b-r" mode="root" />, { wrapper: TooltipProvider });
    expect(screen.getByRole("heading", { level: 2, name: "ص ب ر" })).toBeInTheDocument();
    expect(screen.getByText("Occurrences: 103 · Verses: 90")).toBeInTheDocument();
  });

  it("offers a choice in root mode when the query means several", () => {
    classified = { kind: "latin_root" };
    rootSearch = { roots: [ROOT, { id: 8, root: "س ب ر", occurrences: 1 }], lemmas: [] };
    render(<SearchView q="s-b-r" mode="root" />, { wrapper: TooltipProvider });
    expect(screen.getAllByRole("link", { name: /ب ر/ })).toHaveLength(2);
  });

  it("searches the translations for a Latin root in Text mode", () => {
    classified = { kind: "latin_root" };
    render(<SearchView q="s-b-r" mode="text" />, { wrapper: TooltipProvider });
    expect(searches[0]?.scope).toBe("translations");
  });

  it("shows verses similar in meaning under the matches in All mode", () => {
    semanticStatus = { state: "ready", pack: "vectors.bge-m3", provider: null };
    render(<SearchView q="patience" mode="all" />, { wrapper: TooltipProvider });
    expect(meaningSearches[0]).toMatchObject({ query: "patience", limit: 5 });
    const group = screen.getByRole("heading", { name: "Similar in meaning" }).closest("section");
    if (!group) throw new Error("no group");
    expect(within(group).getByRole("link", { name: /Seek help in patience/ })).toHaveAttribute(
      "href",
      "/read/2/45",
    );
    expect(
      within(group).getByRole("link", { name: "More verses similar in meaning" }),
    ).toHaveAttribute("href", "/search?q=patience&mode=meaning");
  });

  it("names what is missing instead of verses similar in meaning", () => {
    render(<SearchView q="patience" mode="all" />, { wrapper: TooltipProvider });
    expect(meaningSearches).toHaveLength(0);
    expect(
      screen.getByText("Searching by meaning needs the semantic search packs."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Packs" })).toBeInTheDocument();
  });

  it("lists verses by meaning in Meaning mode and asks for more", () => {
    semanticStatus = { state: "ready", pack: "vectors.bge-m3", provider: null };
    meaning = Array.from({ length: 20 }, (_, index) => ({ ...SIMILAR, ayah: index + 1 }));
    render(<SearchView q="patience" mode="meaning" />, { wrapper: TooltipProvider });
    expect(searches).toHaveLength(0);
    expect(meaningSearches.at(-1)).toMatchObject({ query: "patience", limit: 20 });
    expect(screen.getAllByRole("link", { name: /Seek help/ })).toHaveLength(20);
    fireEvent.click(screen.getByRole("button", { name: "Show more" }));
    expect(meaningSearches.at(-1)).toMatchObject({ limit: 40 });
    expect(screen.queryByText(/are sent to/)).not.toBeInTheDocument();
  });

  it("says where queries go when an API pack is active", () => {
    semanticStatus = { state: "ready", pack: "vectors.voyage", provider: "voyage" };
    meaning = [SIMILAR];
    render(<SearchView q="patience" mode="meaning" />, { wrapper: TooltipProvider });
    expect(screen.getByText(/Your searches are sent to Voyage AI\./)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Show more" })).not.toBeInTheDocument();
  });

  it("names the missing model pack in Meaning mode", () => {
    semanticStatus = {
      state: "model_pack_missing",
      pack: "vectors.bge-m3",
      model_pack: "model.bge-m3",
    };
    render(<SearchView q="patience" mode="meaning" />, { wrapper: TooltipProvider });
    expect(
      screen.getByText("The semantic search pack needs its model pack, model.bge-m3."),
    ).toBeInTheDocument();
    expect(meaningSearches).toHaveLength(0);
  });
});
