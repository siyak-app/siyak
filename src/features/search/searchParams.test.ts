import { describe, expect, it } from "vitest";

import { parseSearchParams } from "./searchParams";

describe("parseSearchParams", () => {
  it("keeps a valid query and mode", () => {
    expect(parseSearchParams({ q: "sabır", mode: "text" })).toEqual({ q: "sabır", mode: "text" });
  });

  it("falls back for anything unusable", () => {
    expect(parseSearchParams({})).toEqual({ q: "", mode: "all" });
    expect(parseSearchParams({ q: ["x"], mode: "root" })).toEqual({ q: "", mode: "all" });
    expect(parseSearchParams({ q: 36 })).toEqual({ q: "36", mode: "all" });
  });
});
