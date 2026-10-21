import { describe, expect, it } from "vitest";

import type { TranslationInfo } from "@/lib/bindings";

import { defaultTranslations, PROFILE_KEYS, profileDefaults } from "./profileDefaults";

const translation = (id: string, lang: string, bundled: boolean): TranslationInfo => ({
  id,
  title: id,
  titles: { tr: null, en: null },
  lang,
  dir: "ltr",
  bundled,
});

const INSTALLED = [
  translation("en.pickthall", "en", true),
  translation("en.sahih", "en", false),
  translation("tr.diyanet", "tr", false),
];

const PANEL = { size: 360, collapsed: false, tab: "tafsir" } as const;

describe("profileDefaults", () => {
  it("sets every profile key", () => {
    for (const profile of ["reader", "researcher"] as const) {
      expect(Object.keys(profileDefaults(profile, INSTALLED, "en", PANEL)).sort()).toEqual(
        [...PROFILE_KEYS].sort(),
      );
    }
  });

  it("gives the reader profile simple defaults", () => {
    expect(profileDefaults("reader", INSTALLED, "en", PANEL)).toEqual({
      experience_profile: "reader",
      "layout.right_panel": { size: 360, collapsed: true, tab: "tafsir" },
      "reader.translations": ["en.pickthall"],
      "reader.actions": "hover",
      "word.click": "card",
      "word.underline": "none",
      "ui.technical_terms": "tooltip",
      "search.mode_chips": false,
      "reader.citation_action": false,
    });
  });

  it("gives the researcher profile the detailed defaults", () => {
    expect(profileDefaults("researcher", INSTALLED, "tr", PANEL)).toEqual({
      experience_profile: "researcher",
      "layout.right_panel": { size: 360, collapsed: false, tab: "word" },
      "reader.translations": ["tr.diyanet", "en.pickthall"],
      "reader.actions": "labeled",
      "word.click": "panel",
      "word.underline": "dotted",
      "ui.technical_terms": "inline",
      "search.mode_chips": true,
      "reader.citation_action": true,
    });
  });
});

describe("defaultTranslations", () => {
  it("prefers the interface language, then bundled packs", () => {
    expect(defaultTranslations(INSTALLED, 2, "en")).toEqual(["en.pickthall", "en.sahih"]);
    expect(defaultTranslations(INSTALLED, 1, "tr")).toEqual(["tr.diyanet"]);
  });

  it("returns what is installed when there are fewer packs than asked", () => {
    expect(defaultTranslations(INSTALLED.slice(0, 1), 2, "tr")).toEqual(["en.pickthall"]);
    expect(defaultTranslations([], 2, "en")).toEqual([]);
  });
});
