import type { AudioPack } from "@/lib/bindings";
import { useAudioPacks } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

const NO_PACKS: readonly AudioPack[] = [];

/** The installed reciters and the one the player uses: the chosen one, else the first. */
export function useReciter(): { packs: readonly AudioPack[]; active: AudioPack | null } {
  const packs = useAudioPacks().data ?? NO_PACKS;
  const chosen = useUiStore((state) => state.preferences["audio.reciter"]);
  const active = packs.find((pack) => pack.id === chosen) ?? packs[0] ?? null;
  return { packs, active };
}

/** A reciter's name in the interface language. */
export function reciterTitle(pack: AudioPack, language: string | undefined): string {
  const localized = language === "tr" ? pack.titles.tr : pack.titles.en;
  return localized ?? pack.title;
}
