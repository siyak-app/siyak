import { create } from "zustand";

import { startTrack, type Track } from "./playback";

/** What the player is doing. */
export type PlayerStatus = "loading" | "playing" | "paused" | "error";

interface AudioState {
  /** The track loaded in the player; `null` when stopped (the bar is hidden). */
  track: Track | null;
  status: PlayerStatus;
  /** Plays a verse from the start, with its basmala if it opens a surah. */
  playVerse: (surah: number, ayah: number) => void;
  /** Loads another track and plays it. */
  setTrack: (track: Track) => void;
  setStatus: (status: PlayerStatus) => void;
  /** Pauses a playing track and resumes a paused one. */
  toggle: () => void;
  stop: () => void;
}

/**
 * The recitation player's state (ADR 0014). Not persisted. `AudioPlayer` owns
 * the `<audio>` element and follows this store; the reader, the verse actions
 * and the bar only change it.
 */
export const useAudioStore = create<AudioState>()((set) => ({
  track: null,
  status: "paused",
  playVerse: (surah, ayah) => {
    set({ track: startTrack(surah, ayah), status: "loading" });
  },
  setTrack: (track) => {
    set({ track, status: "loading" });
  },
  setStatus: (status) => {
    set({ status });
  },
  toggle: () => {
    set((state) => {
      if (!state.track) return state;
      return {
        status: state.status === "playing" || state.status === "loading" ? "paused" : "loading",
      };
    });
  },
  stop: () => {
    set({ track: null, status: "paused" });
  },
}));

/** Whether the player has `surah:ayah` loaded (not its basmala). */
export function isTrackOf(track: Track | null, surah: number, ayah: number): boolean {
  return track !== null && !track.basmala && track.surah === surah && track.ayah === ayah;
}

/** Plays a verse, or pauses or resumes it if it is the one loaded. */
export function playOrToggle(surah: number, ayah: number): void {
  const state = useAudioStore.getState();
  if (
    isTrackOf(state.track, surah, ayah) ||
    (state.track?.basmala && state.track.surah === surah && ayah === 1)
  ) {
    state.toggle();
  } else {
    state.playVerse(surah, ayah);
  }
}
