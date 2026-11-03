import { describe, expect, it } from "vitest";

import type { Surah } from "@/lib/bindings";

import { editionPage, verseIdOf, verseRange } from "./tafsirRefs";

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
