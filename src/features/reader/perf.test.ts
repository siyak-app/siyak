import { describe, expect, it } from "vitest";

import { frameStats } from "./perf";

describe("frameStats", () => {
  it("summarizes frame gaps", () => {
    const gaps = [...Array.from({ length: 18 }, () => 16), 40, 80];
    expect(frameStats(gaps)).toEqual({
      frames: 20,
      meanMs: 20.4,
      p95Ms: 40,
      maxMs: 80,
      slowFrames: 1,
    });
  });

  it("handles no frames", () => {
    expect(frameStats([])).toEqual({ frames: 0, meanMs: 0, p95Ms: 0, maxMs: 0, slowFrames: 0 });
  });
});
