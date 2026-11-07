import { describe, expect, it } from "vitest";

import { linkCitations } from "./citations";

describe("linkCitations", () => {
  it("links single verses and ranges to their first verse", () => {
    expect(linkCitations("Patience [2:153] and [2:155-157].")).toBe(
      "Patience [2:153](verse:2:153) and [2:155-157](verse:2:155).",
    );
  });

  it("leaves other brackets alone", () => {
    expect(linkCitations("[note] [2:x] [1234:1]")).toBe("[note] [2:x] [1234:1]");
  });
});
