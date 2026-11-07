import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

import { announce } from "@/lib/announce";
import { useSurahs } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { audioUrl, nextTrack, type Track, trackVerseId } from "./playback";
import { useAudioStore } from "./store";
import { useReciter } from "./useReciter";

/** The file address of `track` for `packId`, if the verse exists. */
function trackUrl(packId: string, counts: readonly number[], track: Track | null): string | null {
  if (!track) return null;
  const id = trackVerseId(counts, track);
  return id === null ? null : audioUrl(packId, id);
}

/**
 * The recitation player (ADR 0014): one `<audio>` element following
 * `useAudioStore`, plus a second one that preloads the next verse. Files come
 * from the core's `siyak-audio:` scheme. Mounted once, in the app shell;
 * renders nothing visible.
 */
export function AudioPlayer() {
  const { t } = useTranslation();
  const track = useAudioStore((state) => state.track);
  const status = useAudioStore((state) => state.status);
  const { active } = useReciter();
  const surahs = useSurahs();
  const counts = useMemo(() => surahs.data?.map((s) => s.ayah_count) ?? [], [surahs.data]);
  const autoplay = useUiStore((state) => state.preferences["audio.autoplay"]);
  const repeat = useUiStore((state) => state.preferences["audio.repeat"]);
  const audioRef = useRef<HTMLAudioElement>(null);

  const src = active ? trackUrl(active.id, counts, track) : null;
  const upcoming =
    track && counts[track.surah - 1] !== undefined
      ? nextTrack(track, counts[track.surah - 1] ?? 0, { autoplay, repeat })
      : null;
  const nextSrc = active ? trackUrl(active.id, counts, upcoming) : null;

  // Load the track and play or pause it as the store says.
  useEffect(() => {
    const element = audioRef.current;
    if (!element) return;
    if (!src) {
      if (element.getAttribute("src")) {
        element.pause();
        element.removeAttribute("src");
        element.load();
      }
      return;
    }
    if (element.getAttribute("src") !== src) element.src = src;
    if (status === "loading") {
      element.play().catch((error: unknown) => {
        // A newer src or a pause interrupted this play; not a failure.
        if (error instanceof DOMException && error.name === "AbortError") return;
        console.error("Playing the recitation failed", error);
        useAudioStore.getState().setStatus("error");
      });
    } else if (status === "paused") {
      element.pause();
    }
  }, [src, status]);

  // No reciter (the pack was removed): stop.
  useEffect(() => {
    if (track && !active) useAudioStore.getState().stop();
  }, [track, active]);

  const onEnded = () => {
    const element = audioRef.current;
    const { setTrack, stop } = useAudioStore.getState();
    if (!upcoming) {
      stop();
      return;
    }
    if (nextSrc === src && element) {
      // The same verse again (repeat): restart it in place.
      element.currentTime = 0;
      void element.play();
      return;
    }
    setTrack(upcoming);
  };

  return (
    <>
      <audio
        ref={audioRef}
        preload="auto"
        onPlaying={() => {
          useAudioStore.getState().setStatus("playing");
        }}
        onPause={() => {
          const state = useAudioStore.getState();
          // Paused from outside the app (e.g. the system's media keys).
          if (state.status === "playing" && !audioRef.current?.ended) state.setStatus("paused");
        }}
        onEnded={onEnded}
        onError={() => {
          if (!src) return;
          useAudioStore.getState().setStatus("error");
          announce(t("audio.error"));
        }}
      />
      {nextSrc && nextSrc !== src && <audio preload="auto" src={nextSrc} />}
    </>
  );
}
