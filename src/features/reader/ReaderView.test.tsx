import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, type Mock, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import { useAnnouncement } from "@/lib/announce";
import type { Surah, TranslationInfo, VerseWithTranslations } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";
import { stubLayout } from "@/test/layout";

import { ReaderView } from "./ReaderView";
import { recallTopAyah } from "./scrollMemory";

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
  // Present in every surah list, but without verses in the development data.
  3: { ...surah(3, 200, BASMALA), name_tr: "Âl-i İmrân", name_en: "Aal-i-Imraan" },
  9: { ...surah(9, 129, null), name_tr: "Tevbe", name_en: "At-Tawba" },
};

/** Surahs whose text the (development) data has. */
const WITH_TEXT = new Set([1, 2, 9]);

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

let current: { surah: number; ayah?: number } = { surah: 1 };

vi.mock("@tanstack/react-router", () => ({
  useParams: () => current,
}));

// Query data is referentially stable between renders, as with TanStack Query.
const verseCache = new Map<string, VerseWithTranslations[]>();
function cachedVerses(number: number, ids: readonly string[]) {
  const key = `${String(number)}|${ids.join(",")}`;
  let verses = verseCache.get(key);
  if (!verses) {
    const count = WITH_TEXT.has(number) ? (SURAHS[number]?.ayah_count ?? 0) : 0;
    verses = versesOf(number, count, ids);
    verseCache.set(key, verses);
  }
  return verses;
}

vi.mock("@/lib/queries", () => ({
  useSurah: (number: number) => ({ data: SURAHS[number], isError: false }),
  useTranslations: () => ({ data: PACKS }),
  useVerses: (number: number, ids: readonly string[]) => ({
    data: cachedVerses(number, ids),
    isPending: false,
    isError: false,
  }),
  appErrorCode: () => undefined,
}));

function show(number: number, translations: string[] = ["en.pickthall"], ayah?: number) {
  current = { surah: number, ayah };
  useUiStore.getState().setPreference("reader.translations", translations);
  return render(<ReaderView />, { wrapper: TooltipProvider });
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

  it("reorders the shown translations from the picker", async () => {
    show(1, ["en.pickthall", "ur.test"]);
    fireEvent.click(screen.getByRole("button", { name: "Translations" }));
    fireEvent.click(await screen.findByRole("button", { name: "Move Urdu test up" }));
    expect(useUiStore.getState().preferences["reader.translations"]).toEqual([
      "ur.test",
      "en.pickthall",
    ]);
    const urdu = screen.getByText("ur.test 1:1");
    const english = screen.getByText("en.pickthall 1:1");
    expect(urdu.compareDocumentPosition(english) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(urdu).toHaveAttribute("dir", "rtl");
    expect(english).toHaveAttribute("dir", "ltr");
  });

  it("shows and hides translations from the picker", async () => {
    show(1, ["en.pickthall"]);
    fireEvent.click(screen.getByRole("button", { name: "Translations" }));
    const urdu = await screen.findByRole("checkbox", { name: /Urdu test/ });
    expect(urdu).not.toBeChecked();
    fireEvent.click(urdu);
    expect(await screen.findByText("ur.test 1:1")).toHaveAttribute("lang", "ur");
    fireEvent.click(screen.getByRole("checkbox", { name: /The Meaning of the Glorious Koran/ }));
    expect(useUiStore.getState().preferences["reader.translations"]).toEqual(["ur.test"]);
    expect(screen.queryByText("en.pickthall 1:1")).not.toBeInTheDocument();
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

  it("opens a deep link at its verse and highlights it", async () => {
    show(2, ["en.pickthall"], 255);
    const verse = await screen.findByRole("article", { name: "Verse 2:255" });
    expect(verse).toHaveAttribute("data-highlighted", "true");
    expect(screen.queryByRole("article", { name: "Verse 2:1" })).not.toBeInTheDocument();
  });

  it("reports a verse beyond the end of the surah", () => {
    show(1, ["en.pickthall"], 9);
    expect(screen.getByRole("status")).toHaveTextContent("There is no verse 1:9.");
    expect(screen.getByRole("article", { name: "Verse 1:1" })).toBeInTheDocument();
  });

  it("does not break on a verse the data does not contain", () => {
    // e.g. a last position or user data pointing at 3:10 with the development data
    show(3, ["en.pickthall"], 10);
    expect(screen.getByText(/not available in this build/)).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("stores the verse at the top as the last read position", async () => {
    show(2, ["en.pickthall"], 100);
    await vi.waitFor(
      () => {
        expect(useUiStore.getState().preferences["reader.last_position"]).toEqual({
          surah: 2,
          ayah: 100,
        });
      },
      { timeout: 3000 },
    );
  });

  it("returns to where a surah was left when switching back", async () => {
    const first = show(2, ["en.pickthall"], 200);
    await vi.waitFor(() => {
      expect(recallTopAyah(2)).toBe(200);
    });
    first.unmount();
    show(2);
    expect(await screen.findByRole("article", { name: "Verse 2:200" })).toBeInTheDocument();
    expect(screen.queryByRole("article", { name: "Verse 2:1" })).not.toBeInTheDocument();
  });

  describe("verse actions", () => {
    let writeText: Mock<(text: string) => Promise<void>>;
    beforeEach(() => {
      writeText = vi.fn(() => Promise.resolve());
      Object.defineProperty(navigator, "clipboard", {
        configurable: true,
        value: { writeText },
      });
    });

    const verse = (name: string) => screen.findByRole("article", { name });

    it("copies the reference with the surah name in the interface language", async () => {
      show(2, ["en.pickthall"], 153);
      const article = await verse("Verse 2:153");
      fireEvent.click(within(article).getByRole("button", { name: "Copy reference" }));
      await vi.waitFor(() => {
        expect(writeText).toHaveBeenCalledWith("Al-Baqara 2:153");
      });
      expect(useAnnouncement.getState().message).toBe("Copied");
    });

    it("copies the Arabic text", async () => {
      show(2, ["en.pickthall"], 153);
      const article = await verse("Verse 2:153");
      fireEvent.click(within(article).getByRole("button", { name: "Copy Arabic" }));
      await vi.waitFor(() => {
        expect(writeText).toHaveBeenCalledWith("آية 153");
      });
    });

    it("copies all shown translations, each under its title", async () => {
      show(1, ["ur.test", "en.pickthall"]);
      const article = await verse("Verse 1:2");
      fireEvent.click(within(article).getByRole("button", { name: "Copy translations" }));
      await vi.waitFor(() => {
        expect(writeText).toHaveBeenCalledWith(
          "Urdu test\nur.test 1:2\n\nThe Meaning of the Glorious Koran\nen.pickthall 1:2",
        );
      });
    });

    it("offers no translation copy for Arabic only", async () => {
      show(1, []);
      const article = await verse("Verse 1:1");
      expect(within(article).queryByRole("button", { name: /Copy translation/ })).toBeNull();
      expect(within(article).getByRole("button", { name: "Copy Arabic" })).toBeInTheDocument();
    });

    it("shows labels in labeled mode", async () => {
      useUiStore.getState().setPreference("reader.actions", "labeled");
      show(1);
      const article = await verse("Verse 1:1");
      expect(within(article).getByRole("button", { name: "Copy reference" })).toHaveTextContent(
        "Copy reference",
      );
    });

    it("uses icon buttons in hover mode", async () => {
      show(1);
      const article = await verse("Verse 1:1");
      expect(within(article).getByRole("button", { name: "Copy reference" })).toHaveTextContent("");
    });

    it("announces a failed copy", async () => {
      writeText.mockRejectedValueOnce(new Error("denied"));
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
      show(1);
      const article = await verse("Verse 1:1");
      fireEvent.click(within(article).getByRole("button", { name: "Copy Arabic" }));
      await vi.waitFor(() => {
        expect(useAnnouncement.getState().message).toBe("Could not copy to the clipboard.");
      });
      consoleError.mockRestore();
    });
  });
});
