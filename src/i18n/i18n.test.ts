import { describe, expect, it } from "vitest";

import en from "./en.json";
import { systemLanguage } from "./index";
import tr from "./tr.json";

/** Flattens nested resources into dotted keys, e.g. `reader.verse_label`. */
function keys(resource: object, prefix = ""): string[] {
  return Object.entries(resource).flatMap(([key, value]) =>
    typeof value === "object" && value !== null
      ? keys(value as object, `${prefix}${key}.`)
      : [`${prefix}${key}`],
  );
}

describe("translations", () => {
  it("tr and en have identical key sets", () => {
    expect(keys(tr).sort()).toEqual(keys(en).sort());
  });

  it("has no empty messages", () => {
    for (const resource of [en, tr]) {
      const empty = keys(resource).filter((key) => {
        const value = key
          .split(".")
          .reduce<unknown>((node, part) => (node as Record<string, unknown>)[part], resource);
        return typeof value !== "string" || value.trim() === "";
      });
      expect(empty).toEqual([]);
    }
  });
});

describe("systemLanguage", () => {
  it("prefers Turkish when the system asks for it", () => {
    expect(systemLanguage(["tr-TR", "en-US"])).toBe("tr");
  });

  it("uses the first supported language", () => {
    expect(systemLanguage(["de-DE", "en-GB", "tr"])).toBe("en");
  });

  it("falls back to English", () => {
    expect(systemLanguage(["fr-FR"])).toBe("en");
    expect(systemLanguage([])).toBe("en");
  });
});
