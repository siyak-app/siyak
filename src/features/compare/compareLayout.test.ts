import { describe, expect, it } from "vitest";

import type { TafsirInfo, TafsirPassage, TranslationInfo } from "@/lib/bindings";

import { compareColumns, tafsirCells } from "./compareLayout";

const translation = (id: string) => ({ id }) as TranslationInfo;
const tafsir = (id: string) => ({ id }) as TafsirInfo;
const verses = (surah: number, from: number, to: number) =>
  Array.from({ length: to - from + 1 }, (_, i) => ({ surah, ayah: from + i }));
const passage = (from: number, to: number): TafsirPassage => ({
  id: from,
  verse_from: { surah: 2, ayah: from },
  verse_to: { surah: 2, ayah: to },
  paragraphs: [],
});

describe("compareColumns", () => {
  const translations = [translation("en.a"), translation("en.b")];
  const tafsirs = [tafsir("ar.a"), tafsir("ar.b")];
  const ids = (stored: string[] | null, reader: string[] = ["en.b"]) =>
    compareColumns(stored, reader, translations, tafsirs).map((c) => `${c.kind} ${c.pack.id}`);

  it("defaults to the reader's translations and the first tafsir", () => {
    expect(ids(null)).toEqual(["translation en.b", "tafsir ar.a"]);
    expect(compareColumns(null, [], [], [])).toEqual([]);
  });

  it("keeps the chosen order, mixing kinds, without removed packs", () => {
    expect(ids(["ar.b", "en.a", "ar.gone"])).toEqual(["tafsir ar.b", "translation en.a"]);
    expect(ids([])).toEqual([]);
  });
});

describe("tafsirCells", () => {
  it("spans a passage over the rows of its verses", () => {
    const first = passage(1, 3);
    expect(tafsirCells([first], verses(2, 1, 3))).toEqual([
      { passage: first, span: 3 },
      null,
      null,
    ]);
  });

  it("clamps passages that begin before or end after the range", () => {
    const before = passage(1, 2);
    const after = passage(3, 9);
    expect(tafsirCells([before, after], verses(2, 2, 4))).toEqual([
      { passage: before, span: 1 },
      { passage: after, span: 2 },
      null,
    ]);
  });

  it("gathers consecutive verses without commentary into one gap", () => {
    const only = passage(4, 4);
    expect(tafsirCells([only], verses(2, 1, 5))).toEqual([
      { passage: null, span: 3 },
      null,
      null,
      { passage: only, span: 1 },
      { passage: null, span: 1 },
    ]);
  });

  it("keeps verses of other surahs apart", () => {
    expect(tafsirCells([passage(1, 1)], [{ surah: 3, ayah: 1 }])).toEqual([
      { passage: null, span: 1 },
    ]);
    expect(tafsirCells([], [])).toEqual([]);
  });
});
