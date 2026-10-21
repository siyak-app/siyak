import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Surah, VerseWithTranslations } from "@/lib/bindings";

import { ReaderView } from "./ReaderView";

const FATIHA: Surah = {
  number: 1,
  name_ar: "الفاتحة",
  name_tr: "Fâtiha",
  name_en: "Al-Fatihah",
  revelation_type: "meccan",
  ayah_count: 7,
  bismillah: null,
};

const VERSES: VerseWithTranslations[] = [
  {
    id: 1,
    surah: 1,
    ayah: 1,
    juz: 1,
    text_uthmani: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
    translations: [
      {
        pack_id: "en.pickthall",
        lang: "en",
        text: "In the name of Allah, the Beneficent, the Merciful.",
      },
    ],
  },
  {
    id: 2,
    surah: 1,
    ayah: 2,
    juz: 1,
    text_uthmani: "ٱلْحَمْدُ لِلَّهِ رَبِّ ٱلْعَـٰلَمِينَ",
    translations: [
      { pack_id: "en.pickthall", lang: "en", text: "Praise be to Allah, Lord of the Worlds," },
    ],
  },
];

vi.mock("@tanstack/react-router", () => ({
  useParams: () => ({ surah: 1 }),
}));

vi.mock("@/lib/queries", () => ({
  useSurahs: () => ({ data: [FATIHA] }),
  useVerses: () => ({ data: VERSES, isPending: false, isError: false }),
  appErrorCode: () => undefined,
}));

describe("ReaderView", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("renders every verse with its translation", () => {
    render(<ReaderView />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Al-Fatihah");
    expect(screen.getAllByRole("article")).toHaveLength(2);
    expect(screen.getByRole("article", { name: "Verse 1:2" })).toHaveTextContent(
      "Praise be to Allah, Lord of the Worlds,",
    );
  });

  it("marks Arabic blocks as right-to-left Arabic", () => {
    render(<ReaderView />);
    const arabic = screen.getByText(VERSES[0]?.text_uthmani ?? "");
    expect(arabic).toHaveAttribute("dir", "rtl");
    expect(arabic).toHaveAttribute("lang", "ar");
    expect(screen.getByText("الفاتحة")).toHaveAttribute("dir", "rtl");
  });

  it("marks translations with the pack language", () => {
    render(<ReaderView />);
    expect(screen.getByText(/Beneficent, the Merciful/)).toHaveAttribute("lang", "en");
  });

  it("uses the Turkish surah name in Turkish", async () => {
    await i18n.changeLanguage("tr");
    render(<ReaderView />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Fâtiha");
    expect(screen.getByText("Mekkî")).toBeInTheDocument();
    await i18n.changeLanguage("en");
  });
});
