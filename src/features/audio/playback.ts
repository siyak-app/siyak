/**
 * What the recitation player plays next (ADR 0014). Pure functions, so the
 * order of verses, the basmala and the repeat modes are tested without audio.
 *
 * @packageDocumentation
 */

import type { AudioRepeat } from "@/lib/settings";

/** The scheme the core serves audio pack files under (`src-tauri/src/audio.rs`). */
export const AUDIO_SCHEME = "siyak-audio";

/** One thing to play: a verse, or the basmala read before a surah's first verse. */
export interface Track {
  surah: number;
  ayah: number;
  /** The basmala (al-Fātiḥa 1:1 of the same reciter) before `ayah` 1 of `surah`. */
  basmala: boolean;
}

/** Surahs whose first verse is not preceded by a basmala: al-Fātiḥa (it is 1:1) and at-Tawba. */
const NO_BASMALA = new Set([1, 9]);

/** The first track when a verse is played: its basmala if it opens a surah that has one. */
export function startTrack(surah: number, ayah: number): Track {
  return { surah, ayah, basmala: ayah === 1 && !NO_BASMALA.has(surah) };
}

/** Player settings that decide what follows a track. */
export interface FollowRules {
  /** Play the next verse after one ends. */
  autoplay: boolean;
  repeat: AudioRepeat;
}

/**
 * The track after `track` in a surah of `ayahCount` verses, or `null` to stop.
 * The basmala always leads into its verse; repeating a verse replays it
 * without the basmala; at the end of the surah, `surah` repeat starts over.
 */
export function nextTrack(track: Track, ayahCount: number, rules: FollowRules): Track | null {
  if (track.basmala) return { ...track, basmala: false };
  if (rules.repeat === "verse") return track;
  if (!rules.autoplay) return null;
  if (track.ayah < ayahCount) return { surah: track.surah, ayah: track.ayah + 1, basmala: false };
  if (rules.repeat === "surah") return startTrack(track.surah, 1);
  return null;
}

/** The track the "previous" button goes to: the verse before, or the start of this one. */
export function previousTrack(track: Track): Track {
  if (track.basmala || track.ayah <= 1) return startTrack(track.surah, 1);
  return { surah: track.surah, ayah: track.ayah - 1, basmala: false };
}

/** The track the "next" button goes to, regardless of autoplay; `null` past the last verse. */
export function skipTrack(track: Track, ayahCount: number): Track | null {
  if (track.basmala) return { ...track, basmala: false };
  return track.ayah < ayahCount
    ? { surah: track.surah, ayah: track.ayah + 1, basmala: false }
    : null;
}

/**
 * The global verse id (1..6236) of a verse, from the verse counts of every
 * surah in order; `null` if it does not exist.
 */
export function verseId(ayahCounts: readonly number[], surah: number, ayah: number): number | null {
  const count = ayahCounts[surah - 1];
  if (count === undefined || ayah < 1 || ayah > count) return null;
  let before = 0;
  for (let i = 0; i < surah - 1; i += 1) before += ayahCounts[i] ?? 0;
  return before + ayah;
}

/** The verse id a track plays: the basmala is al-Fātiḥa 1:1, verse 1. */
export function trackVerseId(ayahCounts: readonly number[], track: Track): number | null {
  return track.basmala ? 1 : verseId(ayahCounts, track.surah, track.ayah);
}

/**
 * The address of a verse file of an installed audio pack. Windows' webview
 * reaches custom schemes as `http://<scheme>.localhost`, the others as
 * `<scheme>://localhost` (Tauri's convention).
 */
export function audioUrl(
  packId: string,
  verse: number,
  userAgent: string = typeof navigator === "undefined" ? "" : navigator.userAgent,
): string {
  const path = `${encodeURIComponent(packId)}/${String(verse)}.mp3`;
  return userAgent.includes("Windows")
    ? `http://${AUDIO_SCHEME}.localhost/${path}`
    : `${AUDIO_SCHEME}://localhost/${path}`;
}
