import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { Concordance } from "@/lib/bindings";
import type { ConcordanceVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { ConcordanceView } from "./ConcordanceView";

const requests: ConcordanceVariables[] = [];
let data: Concordance;

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    to,
    params,
    "aria-current": current,
  }: {
    children: ReactNode;
    to: string;
    params: Record<string, string | number>;
    "aria-current"?: "page";
  }) => (
    <a
      aria-current={current}
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
  useConcordance: (variables: ConcordanceVariables) => {
    requests.push(variables);
    return { data, isError: false };
  },
  useSurahs: () => ({
    data: [{ number: 2, name_tr: "Bakara", name_en: "Al-Baqara", name_ar: "البقرة" }],
  }),
  useMorphologyPack: () => ({ data: { id: "morphology.qac", source: "Quranic Arabic Corpus" } }),
  useOpenPackSource: () => ({ mutate: vi.fn() }),
}));

const ROOT = { id: 7, root: "ص ب ر", occurrences: 3 };
const LEMMA = { id: 9, lemma: "صَبْر", occurrences: 2 };
const BASE: Concordance = {
  root: ROOT,
  lemma: null,
  lemmas: [LEMMA, { id: 10, lemma: "عَاد2", occurrences: 1 }],
  occurrences: 3,
  verse_count: 60,
  by_surah: [{ surah: 2, occurrences: 3 }],
  verses: [
    {
      surah: 2,
      ayah: 45,
      words: [
        { text_ar: "وَٱسْتَعِينُوا۟", matched: false },
        { text_ar: "بِٱلصَّبْرِ", matched: true },
      ],
      translation: { pack_id: "en.pickthall", lang: "en", text: "Seek help in patience" },
    },
  ],
};

describe("ConcordanceView", () => {
  beforeEach(async () => {
    requests.length = 0;
    data = BASE;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate({
      ...DEFAULT_PREFERENCES,
      "reader.translations": ["en.pickthall", "tr.diyanet"],
    });
  });

  const renderView = () =>
    render(<ConcordanceView target={{ kind: "root", id: 7 }} />, { wrapper: TooltipProvider });

  it("shows the root, the summary and the source", () => {
    renderView();
    expect(screen.getByRole("heading", { level: 1, name: "ص ب ر" })).toBeInTheDocument();
    expect(screen.getByText("Occurrences: 3 · Verses: 60")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Quranic Arabic Corpus" })).toBeInTheDocument();
  });

  it("asks for the reader's first translation", () => {
    renderView();
    expect(requests[0]?.translationId).toBe("en.pickthall");
    expect(screen.getByText("Seek help in patience")).toBeInTheDocument();
  });

  it("marks the matching word and links the verse", () => {
    renderView();
    expect(screen.getByText("بِٱلصَّبْرِ").tagName).toBe("MARK");
    expect(screen.getByText("Al-Baqara 2:45").closest("a")).toHaveAttribute("href", "/read/2/45");
  });

  it("lists the dictionary forms to narrow down to, raising a homograph digit", () => {
    renderView();
    const forms = within(screen.getByRole("navigation", { name: /Narrow down/ }));
    expect(forms.getByRole("link", { name: "All forms (3)" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(forms.getByRole("link", { name: /صَبْر/ })).toHaveAttribute(
      "href",
      "/concordance/lemma/9",
    );
    expect(forms.getAllByText("2").map((item) => item.tagName)).toContain("SUP");
  });

  it("marks the lemma it is narrowed to", () => {
    data = { ...BASE, lemma: LEMMA };
    renderView();
    const forms = within(screen.getByRole("navigation", { name: /Narrow down/ }));
    expect(forms.getByRole("link", { name: /صَبْر/ })).toHaveAttribute("aria-current", "page");
    expect(forms.getByRole("link", { name: "All forms (3)" })).not.toHaveAttribute("aria-current");
  });

  it("asks for more verses", () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: "Show more" }));
    expect(requests.at(-1)?.limit).toBe(100);
  });

  it("offers no more verses when all are shown", () => {
    data = { ...BASE, verse_count: 1 };
    renderView();
    expect(screen.queryByRole("button", { name: "Show more" })).not.toBeInTheDocument();
  });

  it("holds the distribution in a table too", () => {
    renderView();
    const table = within(screen.getByRole("table", { hidden: true }));
    expect(table.getByRole("cell", { name: "2. Al-Baqara", hidden: true })).toBeInTheDocument();
    expect(table.getByRole("cell", { name: "3", hidden: true })).toBeInTheDocument();
  });
});
