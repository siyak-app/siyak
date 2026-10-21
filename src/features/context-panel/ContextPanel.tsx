import {
  BookOpenTextIcon,
  type LucideIcon,
  MessagesSquareIcon,
  NotebookPenIcon,
  SpellCheckIcon,
  WaypointsIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CONTEXT_TABS, type ContextTab } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

const TAB_ICONS: Record<ContextTab, LucideIcon> = {
  tafsir: BookOpenTextIcon,
  word: SpellCheckIcon,
  related: WaypointsIcon,
  notes: NotebookPenIcon,
  chat: MessagesSquareIcon,
};

const isContextTab = (value: string): value is ContextTab =>
  CONTEXT_TABS.some((tab) => tab === value);

/**
 * Right pane: context for the selected verse or word (spec §13.2).
 * TODO(M4): tafsir. TODO(M5): word. TODO(M6): notes. TODO(M7): related. TODO(M8): chat.
 */
export function ContextPanel() {
  const { t } = useTranslation();
  const panel = useUiStore((state) => state.preferences["layout.right_panel"]);
  const setPreference = useUiStore((state) => state.setPreference);

  return (
    <Tabs
      value={panel.tab}
      onValueChange={(value) => {
        if (isContextTab(value)) setPreference("layout.right_panel", { ...panel, tab: value });
      }}
      aria-label={t("context_panel.label")}
      className="flex h-full flex-col p-3"
    >
      <TabsList className="w-full">
        {CONTEXT_TABS.map((tab) => (
          <TabsTrigger key={tab} value={tab}>
            {t(`context_panel.tabs.${tab}`)}
          </TabsTrigger>
        ))}
      </TabsList>
      {CONTEXT_TABS.map((tab) => {
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
