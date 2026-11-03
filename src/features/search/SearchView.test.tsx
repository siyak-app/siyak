import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { QueryClass, SearchHit, SearchResults } from "@/lib/bindings";
import type { SearchTextVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { SearchView } from "./SearchView";

const navigate = vi.fn();
let classified: QueryClass;
let results: SearchResults;
const searches: SearchTextVariables[] = [];

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

vi.mock("@/lib/queries", () => ({
  useClassifyQuery: () => ({ data: classified, isError: false }),
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
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("searches translations in All mode and links to the verse", () => {
    render(<SearchView q="patience" mode="all" />);
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
    render(<SearchView q="الصابرين" mode="all" />);
    expect(searches[0]?.scope).toBe("arabic");
    const mark = screen.getByText("الصابرين", { selector: "mark" });
    expect(mark.parentElement).toHaveAttribute("dir", "rtl");
    expect(mark.parentElement).toHaveAttribute("lang", "ar");
  });

  it("offers the full list when All shows only the first results", () => {
    results = { total: 40, hits: [TRANSLATION] };
    render(<SearchView q="patience" mode="all" />);
    expect(screen.getByRole("link", { name: "See all 40 results" })).toHaveAttribute(
      "href",
      "/search?q=patience&mode=text",
    );
  });

  it("filters and lengthens the list in Text mode", () => {
    results = { total: 60, hits: [TRANSLATION] };
    render(<SearchView q="patience" mode="text" />);
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
    render(<SearchView q="الصابرين" mode="text" />);
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("says when nothing was found", () => {
    results = { total: 0, hits: [] };
    render(<SearchView q="zzz" mode="all" />);
    expect(screen.getByText(/Nothing found/)).toBeInTheDocument();
  });

  it("does not search for references, bad references, roots or nothing", () => {
    classified = { kind: "reference", surah: 2, ayah: 255, ayah_to: null };
    const { unmount } = render(<SearchView q="2:255" mode="all" />);
    expect(screen.getByRole("link", { name: "Go to 2:255" })).toHaveAttribute(
      "href",
      "/read/2/255",
    );
    unmount();

    classified = { kind: "invalid_reference", reason: "ayah_out_of_range" };
    const second = render(<SearchView q="2:999" mode="all" />);
    expect(screen.getByText("That surah has no such verse.")).toBeInTheDocument();
    second.unmount();

    classified = { kind: "latin_root" };
    const third = render(<SearchView q="s-b-r" mode="all" />);
    expect(screen.getByText("Root search arrives in a later version.")).toBeInTheDocument();
    third.unmount();

    classified = { kind: "empty" };
    render(<SearchView q="" mode="all" />);
    expect(screen.getByText(/Type a word/)).toBeInTheDocument();
    expect(searches).toEqual([]);
  });

  it("shows mode chips only when the setting is on", () => {
    const { unmount } = render(<SearchView q="patience" mode="all" />);
    expect(screen.queryByRole("group", { name: "Search mode" })).not.toBeInTheDocument();
    unmount();

    useUiStore.getState().setPreference("search.mode_chips", true);
    render(<SearchView q="patience" mode="all" />);
    fireEvent.click(screen.getByRole("button", { name: "Text" }));
    expect(navigate).toHaveBeenCalledWith({
      to: "/search",
      search: { q: "patience", mode: "text" },
    });
  });
});
