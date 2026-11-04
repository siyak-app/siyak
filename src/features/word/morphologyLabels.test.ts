import type { TFunction } from "i18next";
import { describe, expect, it } from "vitest";

import en from "@/i18n/en.json";
import tr from "@/i18n/tr.json";

import { featureRows, mainSegment, posLabel, singleTarget, splitLemma } from "./morphologyLabels";

const t = ((key: string) => key) as unknown as TFunction;

/** Every tag of the QAC pack (`SELECT DISTINCT pos FROM word_segments`). */
const POS_TAGS = [
  "N", "PRON", "V", "P", "CONJ", "DET", "PN", "REL", "REM", "NEG", "ACC", "ADJ", "EMPH", "T", "DEM",
  "COND", "INTG", "SUB", "LOC", "RES", "CERT", "VOC", "RSLT", "PRO", "PRP", "CIRC", "SUP", "PREV",
  "FUT", "RET", "EXP", "INC", "CAUS", "IMPV", "EXL", "AMD", "INT", "EXH", "ANS", "SUR", "AVR", "INL",
  "EQ", "COM", "IMPN",
]; // prettier-ignore

/** Every value of the pack, except verb forms, shown as they are (`siyak-data` pack format §7). */
const FEATURE_VALUES = [
  "aspect_PERF", "aspect_IMPF", "aspect_IMPV", "case_GEN", "case_ACC", "case_NOM",
  "derivation_ACT PCPL", "derivation_VN", "derivation_PASS PCPL", "gender_M", "gender_F",
  "mood_IND", "mood_JUS", "mood_SUBJ", "number_P", "number_S", "number_D", "person_3", "person_2",
  "person_1", "special_<in~", "special_kaAn", "special_kaAd", "state_DEF", "state_INDEF", "voice_PASS",
]; // prettier-ignore

describe("morphology labels", () => {
  it("names every tag and value of the pack in both languages", () => {
    for (const resource of [en, tr]) {
      expect(POS_TAGS.filter((tag) => !(tag in resource.word.pos))).toEqual([]);
      expect(FEATURE_VALUES.filter((value) => !(value in resource.word.value))).toEqual([]);
    }
  });

  it("keeps a tag it cannot name", () => {
    expect(posLabel(t, "N")).toBe("word.pos.N");
    expect(posLabel(t, "NEW")).toBe("NEW");
  });

  it("shows features in display order, without the prefix tokens", () => {
    const rows = featureRows(t, [
      { key: "case", value: "GEN" },
      { key: "gender", value: "M" },
      { key: "prefix", value: "bi+" },
      { key: "verb_form", value: "X" },
    ]);
    expect(rows).toEqual([
      { key: "gender", label: "word.feature.gender", value: "word.value.gender_M" },
      { key: "verb_form", label: "word.feature.verb_form", value: "X" },
      { key: "case", label: "word.feature.case", value: "word.value.case_GEN" },
    ]);
  });

  it("raises only a homograph digit", () => {
    expect(splitLemma("عَاد2")).toEqual({ text: "عَاد", index: "2" });
    expect(splitLemma("صَبْر")).toEqual({ text: "صَبْر", index: null });
  });

  it("summarizes a word by its first stem with a root", () => {
    const segment = (kind: "prefix" | "stem", root: boolean) => ({
      kind,
      text_ar: "",
      pos: "N",
      root: root ? { id: 1, root: "ر ح م", occurrences: 1 } : null,
      lemma: null,
      features: [],
    });
    const word = {
      id: 1,
      surah: 1,
      ayah: 1,
      position: 1,
      text_ar: "",
      translit: null,
      gloss_en: null,
      segments: [segment("prefix", false), segment("stem", false), segment("stem", true)],
    };
    expect(mainSegment(word)).toBe(word.segments[2]);
  });

  it("goes straight to a result only when there is one", () => {
    const root = { id: 1, root: "ر ح م", occurrences: 1 };
    const lemma = { id: 2, lemma: "رَحِمَ", occurrences: 1 };
    expect(singleTarget({ roots: [root], lemmas: [lemma] })).toEqual({ kind: "root", id: 1 });
    expect(singleTarget({ roots: [], lemmas: [lemma] })).toEqual({ kind: "lemma", id: 2 });
    expect(singleTarget({ roots: [root, root], lemmas: [] })).toBeNull();
    expect(singleTarget({ roots: [], lemmas: [lemma, lemma] })).toBeNull();
    expect(singleTarget({ roots: [], lemmas: [] })).toBeNull();
  });
});
