import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import i18n from "@/i18n";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { termLabel, TERMS, useTerm } from "./terms";

describe("termLabel", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("shows the plain wording in tooltip mode", () => {
    expect(termLabel(i18n.t, "lemma", "tooltip")).toBe("dictionary form");
  });

  it("adds the technical term in parentheses in inline mode", () => {
    expect(termLabel(i18n.t, "lemma", "inline")).toBe("dictionary form (lemma)");
    expect(termLabel(i18n.t, "concordance", "inline")).toBe("where it occurs (concordance)");
  });

  it("shows a term with identical forms once", () => {
    expect(termLabel(i18n.t, "root", "inline")).toBe("root");
  });

  it("has both forms for every term in both languages", async () => {
    for (const language of ["en", "tr"]) {
      await i18n.changeLanguage(language);
      for (const name of TERMS) {
        expect(i18n.exists(`term.${name}.plain`)).toBe(true);
        expect(i18n.exists(`term.${name}.technical`)).toBe(true);
      }
    }
    expect(termLabel(i18n.t, "lemma", "inline")).toBe("sözlük biçimi (lemma)");
  });
});

describe("useTerm", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("follows ui.technical_terms", () => {
    const { result, rerender } = renderHook(() => useTerm("semantic_search"));
    expect(result.current).toBe("search by meaning");
    useUiStore.getState().setPreference("ui.technical_terms", "inline");
    rerender();
    expect(result.current).toBe("search by meaning (semantic search)");
  });
});
