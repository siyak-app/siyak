import { Outlet } from "@tanstack/react-router";
import { PanelLeftIcon, PanelRightIcon } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { type PanelSize, usePanelRef } from "react-resizable-panels";

import { LiveAnnouncer } from "@/components/LiveAnnouncer";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { AddToCollectionDialog } from "@/features/collections/AddToCollectionDialog";
import { LibraryNav } from "@/features/collections/LibraryNav";
import { CommandPalette } from "@/features/command-palette/CommandPalette";
import { ContextPanel } from "@/features/context-panel/ContextPanel";
import { OnboardingDialog } from "@/features/onboarding/OnboardingDialog";
import { CoreRefreshBanner, DevDataBadge } from "@/features/core-status/CoreStatus";
import { PacksDialog } from "@/features/packs/PacksDialog";
import { usePackEvents } from "@/features/packs/usePackEvents";
import { SurahList } from "@/features/reader/SurahList";
import { SearchBox } from "@/features/search/SearchBox";
import { SettingsDialog } from "@/features/settings/SettingsDialog";
import type { PaneState } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

type PaneKey = "layout.left_pane" | "layout.right_panel";

/**
 * Records a side pane's size and collapsed state. The last expanded size is
 * kept while collapsed so expanding restores it.
 */
function recordPane(key: PaneKey, size: PanelSize): void {
  const { preferences, setPreference } = useUiStore.getState();
  const current: PaneState = preferences[key];
  const collapsed = size.inPixels < 1;
  const next = collapsed ? current.size : Math.round(size.inPixels);
  if (collapsed === current.collapsed && next === current.size) return;
  setPreference(key, { ...preferences[key], collapsed, size: next });
}

/**
 * The three-pane layout of spec §13.2: navigation, content, context.
 * Side panes are collapsible and resizable; their sizes are stored in pixels
 * so they stay stable when the window is resized.
 */
export function AppShell() {
  const { t } = useTranslation();
  // Read once: the panel library owns the live sizes after mount.
  const initial = useUiStore.getState().preferences;
  const left = initial["layout.left_pane"];
  const right = initial["layout.right_panel"];
  const leftRef = usePanelRef();
  const rightRef = usePanelRef();
  usePackEvents();

  // The right panel is also opened and closed from outside the panel itself:
  // by the experience profile, a settings reset or Enter on a verse.
  useEffect(
    () =>
      useUiStore.subscribe((state, previous) => {
        const next = state.preferences["layout.right_panel"];
        if (next.collapsed === previous.preferences["layout.right_panel"].collapsed) return;
        const handle = rightRef.current;
        if (!handle || handle.isCollapsed() === next.collapsed) return;
        if (next.collapsed) {
          handle.collapse();
        } else {
          handle.expand();
          handle.resize(next.size);
        }
      }),
    [rightRef],
  );

  const toggle = (key: PaneKey, panel: typeof leftRef) => {
    const handle = panel.current;
    if (!handle) return;
    if (handle.isCollapsed()) {
      handle.expand();
      handle.resize(useUiStore.getState().preferences[key].size);
    } else {
      handle.collapse();
    }
  };

  return (
    <div className="flex h-screen flex-col">
      <header className="flex h-11 shrink-0 items-center gap-1 border-b px-2">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("layout.toggle_navigation")}
          onClick={() => {
            toggle("layout.left_pane", leftRef);
          }}
        >
          <PanelLeftIcon />
        </Button>
        <span className="ms-1 flex-1 text-sm font-semibold">{t("app.name")}</span>
        <DevDataBadge />
        <SearchBox />
        <CommandPalette />
        <PacksDialog />
        <SettingsDialog />
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("layout.toggle_context_panel")}
          onClick={() => {
            toggle("layout.right_panel", rightRef);
          }}
        >
          <PanelRightIcon />
        </Button>
      </header>
      <CoreRefreshBanner />
      <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
        <ResizablePanel
          id="navigation"
          panelRef={leftRef}
          defaultSize={left.collapsed ? 0 : left.size}
          minSize={200}
          maxSize="40%"
          collapsible
          groupResizeBehavior="preserve-pixel-size"
          onResize={(size) => {
            recordPane("layout.left_pane", size);
          }}
          className="bg-muted/30"
        >
          <div className="flex h-full flex-col">
            <LibraryNav />
            <div className="min-h-0 flex-1">
              <SurahList />
            </div>
          </div>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel id="content" minSize="30%">
          <main className="h-full">
            <Outlet />
          </main>
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel
          id="context"
          panelRef={rightRef}
          defaultSize={right.collapsed ? 0 : right.size}
          minSize={280}
          maxSize="45%"
          collapsible
          groupResizeBehavior="preserve-pixel-size"
          onResize={(size) => {
            recordPane("layout.right_panel", size);
          }}
        >
          <aside className="h-full">
            <ContextPanel />
          </aside>
        </ResizablePanel>
      </ResizablePanelGroup>
      <OnboardingDialog />
      <AddToCollectionDialog />
      <LiveAnnouncer />
    </div>
  );
}
