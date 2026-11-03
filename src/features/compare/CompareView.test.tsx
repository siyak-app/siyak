import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { Compare, TafsirInfo, TranslationInfo } from "@/lib/bindings";
import type { CompareVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { CompareView } from "./CompareView";

const navigate = vi.fn();
const requests: CompareVariables[] = [];
let compare: Compare;

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigate,
  Link: ({ children, className }: { children: ReactNode; className?: string }) => (
    <a className={className}>{children}</a>
  ),
}));

const PICKTHALL: TranslationInfo = {
  id: "en.pickthall",
  title: "Pickthall",
  titles: { tr: null, en: null },
  lang: "en",
  dir: "ltr",
  bundled: true,
};

const JALALAYN: TafsirInfo = {
  id: "ar.jalalayn",
  title: "Jalalayn",
  titles: { tr: null, en: null },
  author: null,
  lang: "ar",
  dir: "rtl",
  attribution: "Source: Quranpedia",
  has_introduction: false,
};

vi.mock("@/lib/queries", () => ({
  useSurahs: () => ({
    data: [
      { number: 1, ayah_count: 7 },
      {
        number: 2,
        ayah_count: 286,
        name_tr: "Bakara",
        name_en: "Al-Baqara",
        name_ar: "البقرة",
      },
    ],
    error: null,
  }),
  useTranslations: () => ({ data: [PICKTHALL], error: null }),
  useTafsirs: () => ({ data: [JALALAYN], error: null }),
  useCompare: (variables: CompareVariables) => {
    requests.push(variables);
    return { data: compare, isError: false };
  },
  appErrorCode: () => undefined,
}));

const verse = (ayah: number, text: string) => ({
  id: 7 + ayah,
  surah: 2,
  ayah,
  juz: 1,
  text_uthmani: `arabic ${String(ayah)}`,
  translations: text ? [{ pack_id: "en.pickthall", lang: "en", text }] : [],
});

function show(from = 153, to = 155) {
  return render(
    <TooltipProvider>
      <CompareView surah={2} from={from} to={to} />
    </TooltipProvider>,
  );
}

describe("CompareView", () => {
  beforeEach(async () => {
    navigate.mockClear();
    requests.length = 0;
    compare = {
      verses: [verse(153, "one"), verse(154, "two"), verse(155, "")],
      tafsirs: [
        {
          pack_id: "ar.jalalayn",
          passages: [
            {
              id: 1,
              verse_from: { surah: 2, ayah: 153 },
              verse_to: { surah: 2, ayah: 154 },
              paragraphs: [{ idx: 0, text: "<b>sharh</b>", part: null, page: null }],
            },
          ],
        },
      ],
    };
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
    useUiStore.getState().setPreference("reader.translations", ["en.pickthall"]);
  });

  it("asks for the range by verse id with the default columns", () => {
    show();
    expect(requests[0]).toEqual({
      from: 160,
      to: 162,
      translationIds: ["en.pickthall"],
      tafsirIds: ["ar.jalalayn"],
    });
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Al-Baqara 2:153–155");
    const headers = screen.getAllByRole("columnheader").map((cell) => cell.textContent);
    expect(headers).toEqual(["Verse", "Pickthall", "Jalalayn"]);
  });

  it("aligns rows per verse and spans a passage over its verses", () => {
    show();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(3);
    const [first, second, third] = rows.map((row) => within(row).getAllByRole("cell"));
    expect(first?.[0]).toHaveTextContent("one");
    expect(first?.[0]).toHaveAttribute("lang", "en");
    expect(first?.[1]).toHaveAttribute("rowspan", "2");
    expect(first?.[1]).toHaveTextContent("This commentary covers verses 2:153–154.");
    expect(first?.[1]?.querySelector("b")).toHaveTextContent("sharh");
    expect(first?.[1]?.querySelector("[dir]")).toHaveAttribute("dir", "rtl");
    expect(second).toHaveLength(1);
    expect(third?.[1]).toHaveTextContent("no separate commentary");
    expect(screen.getByText("Source: Quranpedia")).toBeInTheDocument();
  });

  it("follows the chosen column order", () => {
    useUiStore.getState().setPreference("compare.columns", ["ar.jalalayn", "en.pickthall"]);
    show();
    const headers = screen.getAllByRole("columnheader").map((cell) => cell.textContent);
    expect(headers).toEqual(["Verse", "Jalalayn", "Pickthall"]);
  });

  it("adds, removes and reorders columns from the picker", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Columns" }));
    fireEvent.click(screen.getByRole("button", { name: /Jalalayn.*up|up.*Jalalayn/i }));
    expect(useUiStore.getState().preferences["compare.columns"]).toEqual([
      "ar.jalalayn",
      "en.pickthall",
    ]);
    fireEvent.click(screen.getByRole("checkbox", { name: /Pickthall/ }));
    expect(useUiStore.getState().preferences["compare.columns"]).toEqual(["ar.jalalayn"]);
  });

  it("explains an empty choice", () => {
    useUiStore.getState().setPreference("compare.columns", []);
    show();
    expect(screen.getByText(/No column is chosen/)).toBeInTheDocument();
    expect(requests[0]).toMatchObject({ translationIds: [], tafsirIds: [] });
  });

  it("goes to another range, only a valid one", () => {
    show();
    fireEvent.change(screen.getByLabelText("From verse"), { target: { value: "200" } });
    const submit = screen.getByRole("button", { name: "Show" });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText("To"), { target: { value: "210" } });
    fireEvent.click(submit);
    expect(navigate).toHaveBeenCalledWith({
      to: "/compare/$surah/$range",
      params: { surah: 2, range: { from: 200, to: 210 } },
    });
  });

  it("says when a long range is cut short", () => {
    show(1, 100);
    expect(screen.getByRole("status")).toHaveTextContent("Only the first 3 verses");
  });

  it("reports verses the surah does not have", () => {
    show(280, 290);
    expect(screen.getByText("That surah has no such verse.")).toBeInTheDocument();
    expect(requests).toEqual([]);
  });
});
