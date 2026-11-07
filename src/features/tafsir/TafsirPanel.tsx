import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { AddToCollectionButton } from "@/features/collections/AddToCollectionButton";
import { CopyCitationButton } from "@/features/collections/CopyCitationButton";
import { packTitle } from "@/features/packs/packFormat";
import type { TafsirInfo, TafsirPassage } from "@/lib/bindings";
import { useSurahs, useTafsir, useTafsirIntroduction, useTafsirs } from "@/lib/queries";
import type { VersePosition } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

import { MachineTafsir } from "./MachineTafsir";
import { verseIdOf, verseRange } from "./tafsirRefs";

/**
 * The Tafsir tab (spec §5.2): the chosen tafsir's commentary on the selected
 * verse, or its author's introduction, which is never shown as commentary on 1:1.
 */
export function TafsirPanel({ verse }: { verse: VersePosition | null }) {
  const { t, i18n } = useTranslation();
  const tafsirs = useTafsirs();
  const chosen = useUiStore((state) => state.preferences["tafsir.active"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const setPacksDialogOpen = useUiStore((state) => state.setPacksDialogOpen);
  // Tied to the pack and verse it was opened on, so choosing either one
  // goes back to the commentary.
  const [introduction, setIntroduction] = useState<{
    pack: string;
    verse: VersePosition | null;
  } | null>(null);

  // Remembers the default, so installing another tafsir does not replace it.
  const fallback = tafsirs.data?.[0]?.id;
  useEffect(() => {
    if (chosen === null && fallback !== undefined) setPreference("tafsir.active", fallback);
  }, [chosen, fallback, setPreference]);

  if (tafsirs.isPending) return <Hint>{t("tafsir.loading")}</Hint>;
  if (tafsirs.isError) return <ErrorMessage error={tafsirs.error} />;

  // A removed pack may still be the stored choice.
  const active = tafsirs.data.find((tafsir) => tafsir.id === chosen) ?? tafsirs.data[0];
  if (!active) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-3 p-4 text-center">
        <p className="text-sm text-muted-foreground">{t("tafsir.none_installed")}</p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setPacksDialogOpen(true);
          }}
        >
          {t("tafsir.open_packs")}
        </Button>
      </div>
    );
  }

  const language = i18n.resolvedLanguage;
  const showIntroduction =
    active.has_introduction && introduction?.pack === active.id && introduction.verse === verse;

  return (
    <div className="flex h-full min-h-0 flex-col gap-2 pt-2">
      <Select
        value={active.id}
        onValueChange={(value) => {
          setPreference("tafsir.active", value);
        }}
      >
        <SelectTrigger aria-label={t("tafsir.picker")} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {tafsirs.data.map((tafsir) => (
            <SelectItem key={tafsir.id} value={tafsir.id}>
              {packTitle(tafsir.titles, tafsir.title, language)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {active.has_introduction && (
        <Button
          variant="ghost"
          size="sm"
          className="justify-self-start self-start"
          aria-pressed={showIntroduction}
          onClick={() => {
            setIntroduction(showIntroduction ? null : { pack: active.id, verse });
          }}
        >
          {showIntroduction ? t("tafsir.back_to_commentary") : t("tafsir.introduction")}
        </Button>
      )}
      {showIntroduction ? (
        <Introduction tafsir={active} />
      ) : verse ? (
        <Commentary tafsir={active} verse={verse} />
      ) : (
        <Hint>{t("tafsir.select_verse")}</Hint>
      )}
      <p className="text-xs text-muted-foreground">
        {[active.author, active.attribution].filter(Boolean).join(" · ")}
      </p>
    </div>
  );
}

function Hint({ children }: { children: string }) {
  return <p className="min-h-0 flex-1 p-1 text-sm text-muted-foreground">{children}</p>;
}

function Commentary({ tafsir, verse }: { tafsir: TafsirInfo; verse: VersePosition }) {
  const { t } = useTranslation();
  const surahs = useSurahs();
  if (surahs.isError) return <ErrorMessage error={surahs.error} />;
  if (!surahs.data) return <Hint>{t("tafsir.loading")}</Hint>;
  return <CommentaryOf tafsir={tafsir} verseId={verseIdOf(surahs.data, verse)} />;
}

function CommentaryOf({ tafsir, verseId }: { tafsir: TafsirInfo; verseId: number }) {
  const { t } = useTranslation();
  const passage = useTafsir(verseId, tafsir.id);
  if (passage.isError) return <ErrorMessage error={passage.error} />;
  if (passage.isPending) return <Hint>{t("tafsir.loading")}</Hint>;
  if (!passage.data) return <Hint>{t("tafsir.no_commentary")}</Hint>;
  const { verse_from: from, verse_to: to } = passage.data;
  const isRange = from.surah !== to.surah || from.ayah !== to.ayah;
  return (
    <Passage
      tafsir={tafsir}
      passage={passage.data}
      verseId={verseId}
      note={isRange ? t("tafsir.covers", { range: verseRange(from, to) }) : null}
    />
  );
}

function Introduction({ tafsir }: { tafsir: TafsirInfo }) {
  const { t } = useTranslation();
  const passage = useTafsirIntroduction(tafsir.id);
  if (passage.isError) return <ErrorMessage error={passage.error} />;
  if (passage.isPending) return <Hint>{t("tafsir.loading")}</Hint>;
  if (!passage.data) return <Hint>{t("tafsir.no_commentary")}</Hint>;
  return <Passage tafsir={tafsir} passage={passage.data} note={null} />;
}

/** The text of a passage; a new passage starts scrolled to its top. */
function Passage({
  tafsir,
  passage,
  note,
  verseId,
}: {
  tafsir: TafsirInfo;
  passage: TafsirPassage;
  note: string | null;
  /** The verse being read, for commentary: an introduction belongs to no verse. */
  verseId?: number;
}) {
  const { t } = useTranslation();
  const citationAction = useUiStore((state) => state.preferences["reader.citation_action"]);
  return (
    <>
      {verseId !== undefined && (
        <div className="flex flex-wrap gap-1">
          <AddToCollectionButton item={{ kind: "tafsir", pack_id: tafsir.id, verse: verseId }} />
          {citationAction && (
            <CopyCitationButton
              reference={{ kind: "tafsir", pack_id: tafsir.id, verse: verseId }}
            />
          )}
        </div>
      )}
      <section
        key={passage.id}
        tabIndex={0}
        aria-label={t("tafsir.text_label")}
        className="min-h-0 flex-1 overflow-y-auto rounded-md p-1 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
      >
        {note && <p className="pb-2 text-xs text-muted-foreground">{note}</p>}
        <MachineTafsir tafsir={tafsir} passage={passage} />
      </section>
    </>
  );
}
