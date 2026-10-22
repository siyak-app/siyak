import { describe, expect, it } from "vitest";

import { moveTranslation, toggleTranslation } from "./translationOrder";

describe("toggleTranslation", () => {
  it("appends a newly shown translation", () => {
    expect(toggleTranslation(["en.a"], "tr.b", true)).toEqual(["en.a", "tr.b"]);
  });

  it("removes a hidden translation", () => {
    expect(toggleTranslation(["en.a", "tr.b"], "en.a", false)).toEqual(["tr.b"]);
  });

  it("keeps the place of an already shown translation", () => {
    expect(toggleTranslation(["en.a", "tr.b"], "en.a", true)).toEqual(["en.a", "tr.b"]);
  });
});

describe("moveTranslation", () => {
  const list = ["a", "b", "c"];

  it("moves up and down", () => {
    expect(moveTranslation(list, "b", -1)).toEqual(["b", "a", "c"]);
    expect(moveTranslation(list, "b", 1)).toEqual(["a", "c", "b"]);
  });

  it("stays put at either end or for an unknown id", () => {
    expect(moveTranslation(list, "a", -1)).toEqual(list);
    expect(moveTranslation(list, "c", 1)).toEqual(list);
    expect(moveTranslation(list, "x", 1)).toEqual(list);
  });

  it("does not modify its input", () => {
    moveTranslation(list, "b", -1);
    expect(list).toEqual(["a", "b", "c"]);
  });
});
