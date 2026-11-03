import {
  type LucideIcon,
  MessagesSquareIcon,
  NotebookPenIcon,
  SpellCheckIcon,
  WaypointsIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TafsirPanel } from "@/features/tafsir/TafsirPanel";
import { surahName } from "@/features/reader/surahName";
import { verseNumber, verseReference } from "@/features/reader/verseCopy";
import { useSurahs } from "@/lib/queries";
import { CONTEXT_TABS, type ContextTab } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

const TAB_ICONS: Record<Exclude<ContextTab, "tafsir">, LucideIcon> = {
  word: SpellCheckIcon,
  related: WaypointsIcon,
  notes: NotebookPenIcon,
  chat: MessagesSquareIcon,
};

const isContextTab = (value: string): value is ContextTab =>
  CONTEXT_TABS.some((tab) => tab === value);

/**
 * Right pane: context for the selected verse or word (spec §13.2).
 * TODO(M5): word. TODO(M6): notes. TODO(M7): related. TODO(M8): chat.
 */
export function ContextPanel() {
  const { t, i18n } = useTranslation();
  const panel = useUiStore((state) => state.preferences["layout.right_panel"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const selected = useUiStore((state) => state.selectedVerse);
  const surahs = useSurahs();
  const surah = selected && surahs.data?.find((item) => item.number === selected.surah);

  return (
    <Tabs
      value={panel.tab}
      onValueChange={(value) => {
        if (isContextTab(value)) setPreference("layout.right_panel", { ...panel, tab: value });
      }}
      aria-label={t("context_panel.label")}
      className="flex h-full flex-col p-3"
    >
      {selected && (
        <p className="px-1 pb-3 text-sm">
          <span className="text-muted-foreground">{t("context_panel.selected_verse")} </span>
          <span className="font-medium">
            {surah
              ? verseReference(surahName(surah, i18n.resolvedLanguage), selected)
              : verseNumber(selected)}
          </span>
        </p>
      )}
      <TabsList className="w-full">
        {CONTEXT_TABS.map((tab) => (
          <TabsTrigger key={tab} value={tab}>
            {t(`context_panel.tabs.${tab}`)}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabsContent value="tafsir" className="flex min-h-0 flex-1 flex-col">
        <TafsirPanel verse={selected} />
      </TabsContent>
      {CONTEXT_TABS.filter((tab) => tab !== "tafsir").map((tab) => {
        const Icon = TAB_ICONS[tab];
        return (
          <TabsContent
            key={tab}
            value={tab}
            className="flex flex-1 flex-col items-center justify-center gap-3 text-center text-muted-foreground"
          >
            <Icon aria-hidden className="size-8 opacity-60" />
            <p className="text-sm">{t("context_panel.coming_soon")}</p>
          </TabsContent>
        );
      })}
    </Tabs>
  );
}
