import { describe, expect, it } from "vitest";

import { formatCost } from "./formatCost";

describe("formatCost", () => {
  it("shows small costs in cents and larger ones in dollars", () => {
    expect(formatCost(0.003, "en")).toBe("0.3 ¢");
    expect(formatCost(0.0004, "en")).toBe("< 0.1 ¢");
    expect(formatCost(0, "en")).toBe("$0.00");
    expect(formatCost(0.125, "en")).toBe("$0.125");
    expect(formatCost(12.5, "en")).toBe("$12.50");
    expect(formatCost(0.0035, "tr")).toBe("0,4 ¢");
  });
});
