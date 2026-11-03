import { describe, expect, it } from "vitest";

import type { Catalog, PackView } from "@/lib/bindings";

import { formatBytes, jobFraction, packTitle, withJob } from "./packFormat";

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
