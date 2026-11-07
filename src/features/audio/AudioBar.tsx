import { useNavigate } from "@tanstack/react-router";
import {
  LoaderCircleIcon,
  PauseIcon,
  PlayIcon,
  Repeat1Icon,
  RepeatIcon,
  SkipBackIcon,
  SkipForwardIcon,
  TriangleAlertIcon,
  XIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { surahName } from "@/features/reader/surahName";
import { useSurahs } from "@/lib/queries";
import { AUDIO_REPEAT, type AudioRepeat } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { previousTrack, skipTrack } from "./playback";
import { useAudioStore } from "./store";
import { reciterTitle, useReciter } from "./useReciter";

/** The repeat mode after `mode` (off → verse → surah → off). */
function nextRepeat(mode: AudioRepeat): AudioRepeat {
  const index = AUDIO_REPEAT.indexOf(mode);
  return AUDIO_REPEAT[(index + 1) % AUDIO_REPEAT.length] ?? "off";
}

/** An icon button with a tooltip. */
function BarButton({
  label,
  onClick,
  disabled,
  pressed,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  pressed?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={label}
          aria-pressed={pressed}
          disabled={disabled}
          onClick={onClick}
          className={pressed ? "text-primary" : undefined}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="top">{label}</TooltipContent>
    </Tooltip>
  );
}

/**
 * The recitation bar below the content (ADR 0014): play and pause, previous
 * and next verse, the reciter, repeat and stop, with the verse playing. Shown
 * only while a verse is loaded, so it never takes room while not listening.
 */
export function AudioBar() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const track = useAudioStore((state) => state.track);
  const status = useAudioStore((state) => state.status);
  const { packs, active } = useReciter();
  const surahs = useSurahs();
  const repeat = useUiStore((state) => state.preferences["audio.repeat"]);
  const setPreference = useUiStore((state) => state.setPreference);
  if (!track || !active) return null;

  const surah = surahs.data?.[track.surah - 1];
  const name = surah ? surahName(surah, i18n.resolvedLanguage) : String(track.surah);
  const ayahCount = surah?.ayah_count ?? track.ayah;
  const { setTrack, toggle, stop } = useAudioStore.getState();
  const playing = status === "playing" || status === "loading";
  const repeatLabel = t(`audio.repeat_${repeat}`);

  return (
    <section
      aria-label={t("audio.player")}
      className="flex shrink-0 flex-wrap items-center gap-1 border-t bg-background px-3 py-1.5"
    >
      <BarButton
        label={t("audio.previous")}
        onClick={() => {
          setTrack(previousTrack(track));
        }}
      >
        <SkipBackIcon />
      </BarButton>
      <BarButton label={playing ? t("audio.pause") : t("audio.play")} onClick={toggle}>
        {status === "loading" ? (
          <LoaderCircleIcon className="animate-spin motion-reduce:animate-none" />
        ) : playing ? (
          <PauseIcon />
        ) : (
          <PlayIcon />
        )}
      </BarButton>
      <BarButton
        label={t("audio.next")}
        disabled={skipTrack(track, ayahCount) === null}
        onClick={() => {
          const next = skipTrack(track, ayahCount);
          if (next) setTrack(next);
        }}
      >
        <SkipForwardIcon />
      </BarButton>

      <button
        type="button"
        className="ms-1 truncate rounded px-1 text-sm tabular-nums hover:bg-muted"
        onClick={() => {
          void navigate({
            to: "/read/$surah/$ayah",
            params: { surah: track.surah, ayah: track.ayah },
          });
        }}
      >
        {track.basmala
          ? t("audio.basmala_of", { name })
          : t("audio.now_playing", { name, ayah: track.ayah })}
      </button>
      {status === "error" && (
        <span role="status" className="flex items-center gap-1 text-xs text-destructive">
          <TriangleAlertIcon aria-hidden className="size-3.5" />
          {t("audio.error")}
        </span>
      )}

      <div className="ms-auto flex items-center gap-1">
        <Select
          value={active.id}
          onValueChange={(id) => {
            setPreference("audio.reciter", id);
          }}
        >
          <SelectTrigger size="sm" aria-label={t("audio.reciter")} className="max-w-56">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {packs.map((pack) => (
              <SelectItem key={pack.id} value={pack.id}>
                {reciterTitle(pack, i18n.resolvedLanguage)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <BarButton
          label={repeatLabel}
          pressed={repeat !== "off"}
          onClick={() => {
            setPreference("audio.repeat", nextRepeat(repeat));
          }}
        >
          {repeat === "verse" ? <Repeat1Icon /> : <RepeatIcon />}
        </BarButton>
        <BarButton label={t("audio.stop")} onClick={stop}>
          <XIcon />
        </BarButton>
      </div>
      {active.attribution && (
        <p className="w-full truncate text-[11px] text-muted-foreground" title={active.attribution}>
          {active.attribution}
        </p>
      )}
    </section>
  );
}
