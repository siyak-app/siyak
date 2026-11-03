import { describe, expect, it } from "vitest";

import { parseAyahParam, parseSurahParam, formatRangeParam, parseRangeParam } from "./readParams";

describe("parseSurahParam", () => {
  it("accepts 1 to 114", () => {
    expect(parseSurahParam("1")).toBe(1);
    expect(parseSurahParam("2")).toBe(2);
    expect(parseSurahParam("114")).toBe(114);
  });

  it("rejects anything else", () => {
    for (const raw of ["0", "115", "-1", "2.5", "02", "abc", "", " 2", "2a", "1e2", "Infinity"]) {
      expect(parseSurahParam(raw), raw).toBeNull();
    }
  });
});

describe("parseAyahParam", () => {
  it("accepts verse numbers up to 286", () => {
    expect(parseAyahParam("1")).toBe(1);
    expect(parseAyahParam("255")).toBe(255);
    expect(parseAyahParam("286")).toBe(286);
  });

  it("rejects anything else", () => {
    for (const raw of ["0", "287", "-5", "1.0", "007", "x", ""]) {
      expect(parseAyahParam(raw), raw).toBeNull();
    }
  });
});

describe("parseRangeParam", () => {
  it("accepts one verse or an ascending range", () => {
    expect(parseRangeParam("255")).toEqual({ from: 255, to: 255 });
    expect(parseRangeParam("153-157")).toEqual({ from: 153, to: 157 });
    expect(parseRangeParam("5-5")).toEqual({ from: 5, to: 5 });
  });

  it("rejects anything else", () => {
    for (const raw of ["", "0", "157-153", "1-2-3", "a", "1-", "-1", "287", "1-287"]) {
      expect(parseRangeParam(raw)).toBeNull();
    }
  });

  it("round-trips through formatRangeParam", () => {
    expect(formatRangeParam({ from: 153, to: 157 })).toBe("153-157");
    expect(formatRangeParam({ from: 255, to: 255 })).toBe("255");
  });
});
