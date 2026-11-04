import { describe, expect, it } from "vitest";

import type { VerseWithTranslations } from "@/lib/bindings";

import { translationsText, verseReference } from "./verseCopy";

const verse = (translations: VerseWithTranslations["translations"]): VerseWithTranslations => ({
  id: 160,
  surah: 2,
  ayah: 153,
  juz: 2,
  text_uthmani: "يَـٰٓأَيُّهَا ٱلَّذِينَ ءَامَنُوا۟",
  words: [],
  translations,
});

describe("translationsText", () => {
  it("copies a single translation as is", () => {
    const one = verse([{ pack_id: "en.a", lang: "en", text: "O ye who believe!" }]);
    expect(translationsText(one, { "en.a": "Pickthall" })).toBe("O ye who believe!");
  });

  it("prefixes each of several translations with its title, in display order", () => {
    const two = verse([
      { pack_id: "tr.b", lang: "tr", text: "Ey iman edenler!" },
      { pack_id: "en.a", lang: "en", text: "O ye who believe!" },
    ]);
    expect(translationsText(two, { "en.a": "Pickthall", "tr.b": "Diyanet" })).toBe(
      "Diyanet\nEy iman edenler!\n\nPickthall\nO ye who believe!",
    );
  });

  it("is empty without translations", () => {
    expect(translationsText(verse([]), {})).toBe("");
  });
});

describe("verseReference", () => {
  it("joins the surah name with surah:ayah", () => {
    expect(verseReference("Al-Baqara", { surah: 2, ayah: 153 })).toBe("Al-Baqara 2:153");
    expect(verseReference("Bakara", { surah: 2, ayah: 153 })).toBe("Bakara 2:153");
  });
});
