import { render, screen, within } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Surah, TranslationInfo, VerseWithTranslations } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";
import { stubLayout } from "@/test/layout";

import { ReaderView } from "./ReaderView";

const BASMALA = "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ";

const surah = (number: number, ayahCount: number, bismillah: string | null): Surah => ({
  number,
  name_ar: "سورة",
  name_tr: `Sure ${String(number)}`,
  name_en: `Surah ${String(number)}`,
  revelation_type: "medinan",
  ayah_count: ayahCount,
  bismillah,
});

const SURAHS: Record<number, Surah> = {
  1: { ...surah(1, 7, null), name_tr: "Fâtiha", name_en: "Al-Faatiha", revelation_type: "meccan" },
  2: { ...surah(2, 286, BASMALA), name_tr: "Bakara", name_en: "Al-Baqara" },
  9: { ...surah(9, 129, null), name_tr: "Tevbe", name_en: "At-Tawba" },
};

const PACKS: TranslationInfo[] = [
  {
    id: "en.pickthall",
    title: "The Meaning of the Glorious Koran",
    titles: { tr: "Pickthall İngilizce Meali", en: "The Meaning of the Glorious Koran" },
    lang: "en",
    dir: "ltr",
    bundled: true,
  },
  {
    id: "ur.test",
    title: "Urdu test",
    titles: { tr: null, en: null },
    lang: "ur",
    dir: "rtl",
    bundled: false,
  },
];

const versesOf = (number: number, count: number, ids: readonly string[]): VerseWithTranslations[] =>
  Array.from({ length: count }, (_, i) => ({
    id: number * 1000 + i + 1,
    surah: number,
    ayah: i + 1,
    juz: 1,
    text_uthmani: `آية ${String(i + 1)}`,
    translations: ids.map((id) => ({
      pack_id: id,
      lang: PACKS.find((p) => p.id === id)?.lang ?? null,
      text: `${id} ${String(number)}:${String(i + 1)}`,
    })),
  }));

let current = 1;

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ surah: current }),
}));

vi.mock("@/lib/queries", () => ({
  useSurah: (number: number) => ({ data: SURAHS[number], isError: false }),
  useTranslations: () => ({ data: PACKS }),
  useVerses: (number: number, ids: readonly string[]) => ({
    data: versesOf(number, SURAHS[number]?.ayah_count ?? 0, ids),
    isPending: false,
    isError: false,
  }),
  appErrorCode: () => undefined,
}));

function show(number: number, translations: string[] = ["en.pickthall"]) {
  current = number;
  useUiStore.getState().setPreference("reader.translations", translations);
  return render(<ReaderView />);
}

describe("ReaderView", () => {
  let restoreLayout: () => void;
  beforeAll(() => {
    restoreLayout = stubLayout();
  });
  afterAll(() => {
    restoreLayout();
  });

  beforeEach(async () => {
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("renders verses as a list with accessible names", () => {
    show(1);
    const list = screen.getByRole("list", { name: "Verses of Al-Faatiha" });
    const items = within(list).getAllByRole("listitem");
    expect(items[0]).toHaveAttribute("aria-setsize", "7");
    expect(items[0]).toHaveAttribute("aria-posinset", "1");
    expect(screen.getByRole("article", { name: "Verse 1:1" })).toBeInTheDocument();
    expect(screen.getByText("en.pickthall 1:1")).toBeInTheDocument();
  });

  it("marks Arabic blocks as right-to-left Arabic", () => {
    show(1);
    const arabic = screen.getByText("آية 1");
    expect(arabic).toHaveAttribute("dir", "rtl");
    expect(arabic).toHaveAttribute("lang", "ar");
  });

  it("gives each translation its pack's language and direction", () => {
    show(1, ["ur.test", "en.pickthall"]);
    const urdu = screen.getByText("ur.test 1:1");
    expect(urdu).toHaveAttribute("lang", "ur");
    expect(urdu).toHaveAttribute("dir", "rtl");
    const english = screen.getByText("en.pickthall 1:1");
    expect(english).toHaveAttribute("lang", "en");
    expect(english).toHaveAttribute("dir", "ltr");
    // The selected order is the display order.
    expect(urdu.compareDocumentPosition(english) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("names the translations once, in the surah header", () => {
    show(1, ["en.pickthall", "ur.test"]);
    expect(
      screen.getByText("Translations: The Meaning of the Glorious Koran · Urdu test"),
    ).toBeInTheDocument();
  });

  it("shows the basmala header for al-Baqarah", () => {
    show(2);
    expect(screen.getByTestId("basmala")).toHaveTextContent(BASMALA);
    expect(screen.getByTestId("basmala")).toHaveAttribute("lang", "ar");
  });

  it("has no basmala header for surahs 1 and 9", () => {
    show(1).unmount();
    expect(screen.queryByTestId("basmala")).not.toBeInTheDocument();
    show(9);
    expect(screen.queryByTestId("basmala")).not.toBeInTheDocument();
  });

  it("renders only a window of al-Baqarah's 286 verses", () => {
    show(2);
    const items = within(screen.getByRole("list")).getAllByRole("listitem");
    expect(items.length).toBeGreaterThan(0);
    expect(items.length).toBeLessThan(20);
    expect(items[0]).toHaveAttribute("aria-setsize", "286");
  });

  it("uses the Turkish names in Turkish", async () => {
    await i18n.changeLanguage("tr");
    show(1);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("1. Fâtiha");
    expect(screen.getByText("Meal: Pickthall İngilizce Meali")).toBeInTheDocument();
  });
});
