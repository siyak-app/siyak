/**
 * A development tool for checking that scrolling stays smooth on long
 * surahs. WebKit has no Long Tasks API, so it samples the gaps between
 * animation frames while scrolling the reader from top to bottom.
 *
 * In `pnpm tauri dev`, open a surah (al-Baqarah with `SIYAK_DEV_DATA=full`),
 * open the web inspector and run `await __siyakPerf.scrollTest()`.
 *
 * @packageDocumentation
 */

/** A frame slower than this is visible as a stutter. */
export const SLOW_FRAME_MS = 50;

/** Statistics of the gaps between frames, in milliseconds. */
export interface FrameStats {
  frames: number;
  meanMs: number;
  p95Ms: number;
  maxMs: number;
  /** Frames longer than `SLOW_FRAME_MS`. */
  slowFrames: number;
}

const round = (value: number) => Math.round(value * 10) / 10;

/** Summarizes frame gaps. */
export function frameStats(gaps: readonly number[]): FrameStats {
  if (gaps.length === 0) return { frames: 0, meanMs: 0, p95Ms: 0, maxMs: 0, slowFrames: 0 };
  const sorted = [...gaps].sort((a, b) => a - b);
  const p95 = sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)] ?? 0;
  return {
    frames: gaps.length,
    meanMs: round(gaps.reduce((sum, gap) => sum + gap, 0) / gaps.length),
    p95Ms: round(p95),
    maxMs: round(sorted.at(-1) ?? 0),
    slowFrames: gaps.filter((gap) => gap > SLOW_FRAME_MS).length,
  };
}

const nextFrame = () =>
  new Promise<number>((resolve) => {
    requestAnimationFrame(resolve);
  });

/**
 * Scrolls the open surah from top to bottom, `pixelsPerFrame` at a time,
 * and reports the frame gaps.
 */
export async function scrollTest({ pixelsPerFrame = 120 } = {}): Promise<FrameStats> {
  const scroller = document.querySelector<HTMLElement>("[data-reader-scroll]");
  if (!scroller) throw new Error("Open a surah first.");
  scroller.scrollTop = 0;
  const gaps: number[] = [];
  let last = await nextFrame();
  for (;;) {
    const before = scroller.scrollTop;
    scroller.scrollTop = before + pixelsPerFrame;
    const now = await nextFrame();
    gaps.push(now - last);
    last = now;
    if (scroller.scrollTop <= before) break;
  }
  const stats = frameStats(gaps);
  console.table(stats);
  return stats;
}

declare global {
  interface Window {
    /** Development-only performance tools; see `features/reader/perf.ts`. */
    __siyakPerf?: { scrollTest: typeof scrollTest };
  }
}

/** Exposes the tools as `window.__siyakPerf`. */
export function installPerfTools() {
  window.__siyakPerf = { scrollTest };
}
