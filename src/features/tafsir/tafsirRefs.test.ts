import { describe, expect, it } from "vitest";

import type { Surah } from "@/lib/bindings";

import { editionPage, verseIdOf, verseOfId, verseRange } from "./tafsirRefs";

const surahs = [
  { number: 1, ayah_count: 7 },
  { number: 2, ayah_count: 286 },
  { number: 3, ayah_count: 200 },
] as Surah[];

describe("verseIdOf", () => {
  it("counts the verses of earlier surahs", () => {
    expect(verseIdOf(surahs, { surah: 1, ayah: 1 })).toBe(1);
    expect(verseIdOf(surahs, { surah: 2, ayah: 255 })).toBe(262);
    expect(verseIdOf(surahs, { surah: 3, ayah: 1 })).toBe(294);
  });
});

describe("verseRange", () => {
  it("names the surah once within a surah", () => {
    expect(verseRange({ surah: 2, ayah: 153 }, { surah: 2, ayah: 157 })).toBe("2:153–157");
    expect(verseRange({ surah: 2, ayah: 286 }, { surah: 3, ayah: 1 })).toBe("2:286–3:1");
  });
});

describe("editionPage", () => {
  it("shows what is known of volume and page", () => {
    expect(editionPage("1", 113)).toBe("1/113");
    expect(editionPage(null, 113)).toBe("113");
    expect(editionPage("1", null)).toBe("1");
    expect(editionPage(null, null)).toBeNull();
  });
});

describe("verseOfId", () => {
  it("finds a verse by its global id", () => {
    expect(verseOfId(surahs, 1)).toEqual({ surah: 1, ayah: 1 });
    expect(verseOfId(surahs, 7)).toEqual({ surah: 1, ayah: 7 });
    expect(verseOfId(surahs, 8)).toEqual({ surah: 2, ayah: 1 });
  });

  it("is the inverse of verseIdOf", () => {
    const position = verseOfId(surahs, 100);
    expect(position && verseIdOf(surahs, position)).toBe(100);
  });

  it("knows no verse outside the Quran", () => {
    expect(verseOfId(surahs, 0)).toBeNull();
    expect(verseOfId(surahs, 494)).toBeNull();
  });
});
