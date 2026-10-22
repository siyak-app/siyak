/// <reference types="node" />
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { contrast, over, parseOklch, type Rgba } from "@/test/color";

// Read from disk (Vitest runs from the repository root): Vitest does not load
// CSS, not even with `?raw`.
const css = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

/** The custom properties declared directly in the first `selector { … }` block. */
function tokens(selector: string): Record<string, string> {
  const start = css.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`no ${selector} block in styles.css`);
  const body = css.slice(start, css.indexOf("}", start));
  return Object.fromEntries(
    [...body.matchAll(/--([\w-]+):\s*([^;]+);/gu)].map(([, name = "", value = ""]) => [
      name,
      value,
    ]),
  );
}

const ROOT = tokens(":root");
const THEMES = {
  light: ROOT,
  dark: { ...ROOT, ...tokens(".dark") },
  sepia: { ...ROOT, ...tokens(".sepia") },
};

/** Text (WCAG 1.4.3) needs 4.5:1; focus rings and borders of controls (1.4.11) 3:1. */
const TEXT = 4.5;
const NON_TEXT = 3;

describe.each(Object.entries(THEMES))("%s theme", (_name, theme) => {
  const color = (token: string): Rgba => {
    const value = theme[token];
    if (value === undefined) throw new Error(`--${token} is not defined`);
    return parseOklch(value);
  };
  const background = color("background");

  it.each([
    ["foreground", "background"],
    ["card-foreground", "card"],
    ["popover-foreground", "popover"],
    ["muted-foreground", "background"],
    ["muted-foreground", "muted"],
    ["muted-foreground", "card"],
    ["muted-foreground", "popover"],
    ["primary", "background"],
    ["primary-foreground", "primary"],
    ["secondary-foreground", "secondary"],
    ["accent-foreground", "accent"],
    ["destructive", "background"],
  ])("%s on %s is at least 4.5:1", (text, surface) => {
    expect(contrast(color(text), over(color(surface), background))).toBeGreaterThanOrEqual(TEXT);
  });

  it("keeps translations (foreground at 85%) at least 4.5:1", () => {
    const translation = over({ ...color("foreground"), alpha: 0.85 }, background);
    expect(contrast(translation, background)).toBeGreaterThanOrEqual(TEXT);
  });

  it("keeps muted text readable on the navigation pane (muted at 30%)", () => {
    const pane = over({ ...color("muted"), alpha: 0.3 }, background);
    expect(contrast(color("muted-foreground"), pane)).toBeGreaterThanOrEqual(TEXT);
  });

  it("has a focus ring of at least 3:1", () => {
    expect(contrast(color("ring"), background)).toBeGreaterThanOrEqual(NON_TEXT);
  });
});

describe("contrast", () => {
  it("matches known values", () => {
    const black = parseOklch("oklch(0 0 0)");
    const white = parseOklch("oklch(1 0 0)");
    expect(contrast(black, white)).toBeCloseTo(21, 1);
    expect(contrast(white, white)).toBe(1);
  });

  it("reads alpha", () => {
    expect(parseOklch("oklch(1 0 0 / 10%)").alpha).toBeCloseTo(0.1);
    expect(parseOklch("oklch(1 0 0 / 0.5)").alpha).toBe(0.5);
  });
});
