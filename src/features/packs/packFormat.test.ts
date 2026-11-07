import { describe, expect, it } from "vitest";

import type { Catalog, PackView } from "@/lib/bindings";

import {
  foldForSearch,
  formatBytes,
  groupByLanguage,
  jobFraction,
  matchesQuery,
  orphanedBy,
  packCategory,
  packTitle,
  requiredSize,
  withJob,
} from "./packFormat";

describe("formatBytes", () => {
  it("uses decimal units with one decimal below ten", () => {
    expect(formatBytes(1_900_000, "en")).toBe("1.9 MB");
    expect(formatBytes(1_900_000, "tr")).toBe("1,9 MB");
    expect(formatBytes(42_000_000, "en")).toBe("42 MB");
    expect(formatBytes(512, "en")).toBe("512 byte");
    expect(formatBytes(null, "en")).toBe("0 byte");
  });
});

describe("packTitle", () => {
  it("prefers the interface language and falls back to English", () => {
    expect(packTitle({ tr: "Celâleyn", en: "Jalalayn" }, "x", "tr")).toBe("Celâleyn");
    expect(packTitle({ tr: null, en: null }, "Fallback", "en")).toBe("Fallback");
  });
});

describe("jobs", () => {
  const pack = { id: "ar.jalalayn", job: null } as PackView;
  const catalog = { packs: [pack, { ...pack, id: "other" }] } as Catalog;

  it("replaces the job of one pack only", () => {
    const next = withJob(catalog, "ar.jalalayn", { state: "queued" });
    expect(next.packs.map((p) => p.job)).toEqual([{ state: "queued" }, null]);
    expect(catalog.packs[0]?.job).toBeNull();
  });

  it("reports the fraction of a running job", () => {
    expect(jobFraction({ state: "running", phase: "download", done: 50, total: 200 })).toBe(0.25);
    expect(jobFraction({ state: "running", phase: "verify", done: 0, total: 0 })).toBeNull();
    expect(jobFraction({ state: "queued" })).toBeNull();
  });
});

describe("sections", () => {
  const view = (overrides: Partial<PackView>) =>
    ({
      id: "x",
      kind: "translation",
      lang: null,
      title: { tr: null, en: null },
      title_fallback: "X",
      author: null,
      size: 0,
      installed: false,
      bundled: false,
      requires: [],
      ...overrides,
    }) as PackView;
  const vectors = view({ id: "vectors.a", kind: "vectors", size: 20, requires: ["model.a"] });
  const model = view({ id: "model.a", kind: "model", size: 400 });
  const catalog = { packs: [vectors, model] } as Catalog;

  it("lists a model only when nothing needs it and it is installed", () => {
    expect(packCategory(vectors, catalog)).toBe("vectors");
    expect(packCategory(model, catalog)).toBeNull();
    const orphan = { ...model, installed: true };
    expect(packCategory(orphan, { packs: [orphan] } as Catalog)).toBe("vectors");
    expect(packCategory(view({ kind: "future" }), catalog)).toBe("other");
  });

  it("counts the size of what a pack needs that is not installed", () => {
    expect(requiredSize(vectors, catalog)).toBe(400);
    const installed = { packs: [vectors, { ...model, installed: true }] } as Catalog;
    expect(requiredSize(vectors, installed)).toBe(0);
  });

  it("finds the packs only the removed one needed", () => {
    const both = { ...vectors, installed: true };
    const other = { ...both, id: "vectors.b" };
    const installedModel = { ...model, installed: true };
    expect(orphanedBy(both, { packs: [both, installedModel] } as Catalog)).toEqual([
      installedModel,
    ]);
    expect(orphanedBy(both, { packs: [both, other, installedModel] } as Catalog)).toEqual([]);
  });

  it("searches without case, diacritics or the dotless i", () => {
    expect(foldForSearch("Elmalılı İbn Ṭabarī Qurʾān")).toBe("elmalili ibn tabari quran");
    const tabari = view({ title: { tr: "Taberî", en: "Ṭabarī" }, author: "Ibn Jarir" });
    expect(matchesQuery(tabari, "tabari")).toBe(true);
    expect(matchesQuery(tabari, " jarir ")).toBe(true);
    expect(matchesQuery(tabari, "razi")).toBe(false);
    expect(matchesQuery(tabari, "")).toBe(true);
  });

  it("puts the interface language first", () => {
    const packs = [
      view({ id: "a", lang: null }),
      view({ id: "b", lang: "tr" }),
      view({ id: "c", lang: "en" }),
      view({ id: "d", lang: "ar" }),
    ];
    expect(groupByLanguage(packs, "tr").map(([lang]) => lang)).toEqual(["tr", "ar", "en", null]);
    expect(groupByLanguage(packs, "en").map(([lang]) => lang)).toEqual(["en", "ar", "tr", null]);
  });
});
