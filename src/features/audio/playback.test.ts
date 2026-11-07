import { describe, expect, it } from "vitest";

import {
  audioUrl,
  nextTrack,
  previousTrack,
  skipTrack,
  startTrack,
  trackVerseId,
  verseId,
} from "./playback";

const COUNTS = [7, 286, 200];
const ON = { autoplay: true, repeat: "off" } as const;

describe("playback", () => {
  it("starts a surah with its basmala, except al-Fatiha and at-Tawba", () => {
    expect(startTrack(2, 1)).toEqual({ surah: 2, ayah: 1, basmala: true });
    expect(startTrack(2, 5)).toEqual({ surah: 2, ayah: 5, basmala: false });
    expect(startTrack(1, 1).basmala).toBe(false);
    expect(startTrack(9, 1).basmala).toBe(false);
  });

  it("plays on to the end of the surah, then stops", () => {
    const basmala = startTrack(2, 1);
    const first = nextTrack(basmala, 286, ON);
    expect(first).toEqual({ surah: 2, ayah: 1, basmala: false });
    expect(nextTrack({ surah: 2, ayah: 7, basmala: false }, 286, ON)?.ayah).toBe(8);
    expect(nextTrack({ surah: 2, ayah: 286, basmala: false }, 286, ON)).toBeNull();
  });

  it("stops after one verse without autoplay, but still plays the verse after its basmala", () => {
    const rules = { autoplay: false, repeat: "off" } as const;
    expect(nextTrack({ surah: 2, ayah: 7, basmala: false }, 286, rules)).toBeNull();
    expect(nextTrack(startTrack(2, 1), 286, rules)).toEqual({ surah: 2, ayah: 1, basmala: false });
  });

  it("repeats a verse without its basmala, or the surah from the start", () => {
    const verse = { surah: 2, ayah: 1, basmala: false };
    expect(nextTrack(verse, 286, { autoplay: true, repeat: "verse" })).toEqual(verse);
    const last = { surah: 2, ayah: 286, basmala: false };
    expect(nextTrack(last, 286, { autoplay: true, repeat: "surah" })).toEqual(startTrack(2, 1));
  });

  it("moves back and forward by hand", () => {
    expect(previousTrack({ surah: 2, ayah: 5, basmala: false }).ayah).toBe(4);
    expect(previousTrack({ surah: 2, ayah: 1, basmala: false })).toEqual(startTrack(2, 1));
    expect(skipTrack({ surah: 1, ayah: 7, basmala: false }, 7)).toBeNull();
    expect(skipTrack({ surah: 1, ayah: 6, basmala: false }, 7)?.ayah).toBe(7);
  });

  it("numbers verses globally and plays the basmala as verse 1", () => {
    expect(verseId(COUNTS, 1, 1)).toBe(1);
    expect(verseId(COUNTS, 2, 255)).toBe(262);
    expect(verseId(COUNTS, 2, 287)).toBeNull();
    expect(verseId(COUNTS, 4, 1)).toBeNull();
    expect(trackVerseId(COUNTS, startTrack(2, 1))).toBe(1);
    expect(trackVerseId(COUNTS, { surah: 3, ayah: 1, basmala: false })).toBe(294);
  });

  it("builds the scheme address per platform", () => {
    expect(audioUrl("audio.alafasy", 262, "Mozilla/5.0 (Macintosh)")).toBe(
      "siyak-audio://localhost/audio.alafasy/262.mp3",
    );
    expect(audioUrl("audio.alafasy", 262, "Mozilla/5.0 (Windows NT 10.0)")).toBe(
      "http://siyak-audio.localhost/audio.alafasy/262.mp3",
    );
  });
});
