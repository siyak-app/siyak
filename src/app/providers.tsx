import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { TooltipProvider } from "@/components/ui/tooltip";
import { useUiStore } from "@/stores/ui";

import {
  useApplyLanguage,
  useApplyTheme,
  useApplyTypography,
  usePreferencesSync,
} from "./preferences";

const queryClient = new QueryClient({
  defaultOptions: {
    // Queries read local SQLite; retrying a failure only delays the error message.
    queries: { retry: false, refetchOnWindowFocus: false },
  },
});

function PreferencesGate({ children }: { children: ReactNode }) {
  usePreferencesSync();
  const hydrated = useUiStore((state) => state.hydrated);
  const language = useUiStore((state) => state.preferences["ui.language"]);
  const theme = useUiStore((state) => state.preferences["ui.theme"]);
  useApplyLanguage(language);
  const arabic = useUiStore((state) => state.preferences["typography.arabic"]);
  const translation = useUiStore((state) => state.preferences["typography.translation"]);
  const arabicFont = useUiStore((state) => state.preferences["typography.arabic_font"]);
  const translationFont = useUiStore((state) => state.preferences["typography.translation_font"]);
  useApplyTheme(theme);
  useApplyTypography(arabic, translation, arabicFont, translationFont);
  // Pane sizes are only read on mount, so wait for the stored layout.
  return hydrated ? children : null;
}

/** Application-wide providers: data cache, tooltips and UI preferences. */
export function Providers({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <PreferencesGate>{children}</PreferencesGate>
      </TooltipProvider>
    </QueryClientProvider>
  );
}
