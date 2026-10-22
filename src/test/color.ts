/**
 * Colour maths for the contrast test: CSS `oklch()` to sRGB and the WCAG 2
 * contrast ratio.
 */

/** An sRGB colour with channels in 0..1 and its alpha. */
export interface Rgba {
  r: number;
  g: number;
  b: number;
  alpha: number;
}

const clamp = (value: number) => Math.min(1, Math.max(0, value));

/** Linear-light sRGB to gamma-encoded sRGB. */
const encode = (value: number) =>
  value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055;

/** Gamma-encoded sRGB to linear light. */
const decode = (value: number) =>
  value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;

/** Parses `oklch(L C H)` or `oklch(L C H / A)` (A as a number or percentage). */
export function parseOklch(text: string): Rgba {
  const match = /^oklch\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*(?:\/\s*([\d.]+)(%?))?\s*\)$/u.exec(
    text.trim(),
  );
  if (!match) throw new Error(`not an oklch() colour: ${text}`);
  const [, l, c, h, a, percent] = match;
  const lightness = Number(l);
  const chroma = Number(c);
  const hue = (Number(h) * Math.PI) / 180;
  const alpha = a === undefined ? 1 : Number(a) / (percent ? 100 : 1);

  // OKLab to linear sRGB (Björn Ottosson's reference matrices).
  const okA = chroma * Math.cos(hue);
  const okB = chroma * Math.sin(hue);
  const l_ = (lightness + 0.3963377774 * okA + 0.2158037573 * okB) ** 3;
  const m_ = (lightness - 0.1055613458 * okA - 0.0638541728 * okB) ** 3;
  const s_ = (lightness - 0.0894841775 * okA - 1.291485548 * okB) ** 3;
  return {
    r: clamp(encode(4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_)),
    g: clamp(encode(-1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_)),
    b: clamp(encode(-0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_)),
    alpha,
  };
}

/** `top` drawn over the opaque `bottom`, blended in gamma-encoded sRGB as browsers do. */
export function over(top: Rgba, bottom: Rgba): Rgba {
  const mix = (a: number, b: number) => a * top.alpha + b * (1 - top.alpha);
  return { r: mix(top.r, bottom.r), g: mix(top.g, bottom.g), b: mix(top.b, bottom.b), alpha: 1 };
}

/** WCAG 2 relative luminance. */
function luminance({ r, g, b }: Rgba): number {
  return 0.2126 * decode(r) + 0.7152 * decode(g) + 0.0722 * decode(b);
}

/** WCAG 2 contrast ratio of two opaque colours, 1..21. */
export function contrast(a: Rgba, b: Rgba): number {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (high + 0.05) / (low + 0.05);
}
