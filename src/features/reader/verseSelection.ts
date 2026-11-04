import type { ContextTab } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/** Whether the user has selected some text (a click then only ends the selection). */
export function hasTextSelection(): boolean {
  return (window.getSelection()?.toString() ?? "") !== "";
}

/** Opens the context panel, on `tab` if given. */
export function openContextPanel(tab?: ContextTab) {
  const { preferences, setPreference } = useUiStore.getState();
  const panel = preferences["layout.right_panel"];
  const nextTab = tab ?? panel.tab;
  if (panel.collapsed || nextTab !== panel.tab) {
    setPreference("layout.right_panel", { ...panel, collapsed: false, tab: nextTab });
  }
}
