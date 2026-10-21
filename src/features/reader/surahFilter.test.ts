import { describe, expect, it } from "vitest";

import type { Surah } from "@/lib/bindings";

import { matchesSurah } from "./surahFilter";

const BAQARAH: Surah = {
  number: 2,
  name_ar: "البقرة",
  name_tr: "Bakara",
  name_en: "Al-Baqara",
  revelation_type: "medinan",
  ayah_count: 286,
  bismillah: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
};

const IKHLAS: Surah = {
  number: 112,
  name_ar: "الإخلاص",
  name_tr: "İhlâs",
  name_en: "Al-Ikhlaas",
  revelation_type: "meccan",
  ayah_count: 4,
  bismillah: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
};

describe("matchesSurah", () => {
  it("matches every surah when the filter is empty", () => {
    expect(matchesSurah(BAQARAH, "  ")).toBe(true);
  });

  it("matches the number as a prefix", () => {
    expect(matchesSurah(BAQARAH, "2")).toBe(true);
    expect(matchesSurah(IKHLAS, "11")).toBe(true);
    expect(matchesSurah(IKHLAS, "2")).toBe(false);
  });

  it("matches Turkish and English names in any spelling", () => {
    for (const query of ["bak", "Bakara", "baqarah", "Al-Baqara", "BAQ"]) {
      expect(matchesSurah(BAQARAH, query), query).toBe(true);
    }
    for (const query of ["ihlas", "İhlâs", "ikhlas", "Ikhlaas"]) {
      expect(matchesSurah(IKHLAS, query), query).toBe(true);
    }
    expect(matchesSurah(BAQARAH, "ihlas")).toBe(false);
  });

  it("matches the Arabic name without diacritics or hamza forms", () => {
    expect(matchesSurah(BAQARAH, "بقره")).toBe(true);
    expect(matchesSurah(IKHLAS, "الاخلاص")).toBe(true);
    expect(matchesSurah(IKHLAS, "الْإِخْلَاص")).toBe(true);
  });
});
