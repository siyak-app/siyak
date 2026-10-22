import { describe, expect, it } from "vitest";

import type { Surah } from "@/lib/bindings";

import { parseGoTo } from "./goTo";

const surah = (
  number: number,
  name_tr: string,
  name_en: string,
  name_ar: string,
  ayah_count: number,
): Surah => ({
  number,
  name_tr,
  name_en,
  name_ar,
  ayah_count,
  revelation_type: "medinan",
  bismillah: null,
});

const SURAHS: Surah[] = [
  surah(1, "Fâtiha", "Al-Faatiha", "الفاتحة", 7),
  surah(2, "Bakara", "Al-Baqara", "البقرة", 286),
  surah(3, "Âl-i İmrân", "Aal-i-Imraan", "آل عمران", 200),
  surah(9, "Tevbe", "At-Tawba", "التوبة", 129),
  surah(112, "İhlâs", "Al-Ikhlaas", "الإخلاص", 4),
  surah(113, "Felak", "Al-Falaq", "الفلق", 5),
];

const verse = (surah: number, ayah?: number) => ({
  ok: true,
  target: ayah === undefined ? { surah } : { surah, ayah },
});

describe("parseGoTo", () => {
  it.each([
    "2:255",
    "2 255",
    "2.255",
    " 2 : 255 ",
    "Bakara 255",
    "bakara:255",
    "Baqarah 255",
    "Baqara 255",
    "Al-Baqara:255",
    "al baqara 255",
    "البقرة 255",
    "البقره ٢٥٥",
  ])("reads %j as 2:255", (input) => {
    expect(parseGoTo(input, SURAHS)).toEqual(verse(2, 255));
  });

  it("goes to a whole surah by number or name", () => {
    expect(parseGoTo("2", SURAHS)).toEqual(verse(2));
    expect(parseGoTo("Tevbe", SURAHS)).toEqual(verse(9));
    expect(parseGoTo("at-tawba", SURAHS)).toEqual(verse(9));
    expect(parseGoTo("Al-i Imran 7", SURAHS)).toEqual(verse(3, 7));
    expect(parseGoTo("ihlas", SURAHS)).toEqual(verse(112));
  });

  it("accepts a unique beginning of a name", () => {
    expect(parseGoTo("Bak 255", SURAHS)).toEqual(verse(2, 255));
    // "F" starts both Fâtiha and Felak.
    expect(parseGoTo("F 1", SURAHS)).toEqual({ ok: false, error: "no_match" });
  });

  it("reports surahs out of range", () => {
    expect(parseGoTo("0", SURAHS)).toEqual({ ok: false, error: "surah_out_of_range" });
    expect(parseGoTo("115:1", SURAHS)).toEqual({ ok: false, error: "surah_out_of_range" });
  });

  it("reports verses out of range", () => {
    expect(parseGoTo("2:287", SURAHS)).toEqual({ ok: false, error: "ayah_out_of_range" });
    expect(parseGoTo("Fatiha 8", SURAHS)).toEqual({ ok: false, error: "ayah_out_of_range" });
    expect(parseGoTo("2:0", SURAHS)).toEqual({ ok: false, error: "ayah_out_of_range" });
  });

  it("checks only the surah number while the surah list is not loaded", () => {
    expect(parseGoTo("2:255", [])).toEqual(verse(2, 255));
    expect(parseGoTo("Bakara 255", [])).toEqual({ ok: false, error: "no_match" });
  });

  it("rejects garbage and ignores empty input", () => {
    expect(parseGoTo("xyzzy", SURAHS)).toEqual({ ok: false, error: "no_match" });
    expect(parseGoTo("::", SURAHS)).toEqual({ ok: false, error: "no_match" });
    expect(parseGoTo("2:255:3", SURAHS)).toEqual({ ok: false, error: "no_match" });
    expect(parseGoTo("   ", SURAHS)).toBeNull();
  });
});
