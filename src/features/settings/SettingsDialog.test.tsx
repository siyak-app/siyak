import { fireEvent, render, renderHook, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useApplyTypography } from "@/app/preferences";
import { TooltipProvider } from "@/components/ui/tooltip";
import i18n from "@/i18n";
import type { TranslationInfo } from "@/lib/bindings";
import type { Settings } from "@/lib/settings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { SettingsDialog } from "./SettingsDialog";

const PACKS: TranslationInfo[] = [
  {
    id: "en.pickthall",
    title: "The Meaning of the Glorious Koran",
    titles: { tr: null, en: null },
    lang: "en",
    dir: "ltr",
    bundled: true,
  },
  {
    id: "tr.test",
    title: "Türkçe deneme",
    titles: { tr: null, en: null },
    lang: "tr",
    dir: "ltr",
    bundled: false,
  },
];

const saved: Partial<Settings>[] = [];

vi.mock("@/lib/queries", () => ({
  useTranslations: () => ({ data: PACKS, isPending: false }),
  useAudioPacks: () => ({ data: [] }),
  useSetSettings: () => ({
    mutate: (values: Partial<Settings>, options: { onSuccess: () => void }) => {
      saved.push(values);
      options.onSuccess();
    },
    isPending: false,
    isError: false,
  }),
  useCitationStyles: () => ({
    data: [
      { id: "isnad", name: "İSNAD" },
      { id: "apa", name: "APA" },
    ],
  }),
  useExportUserData: () => ({ mutate: vi.fn(), isPending: false, isError: false }),
  useImportUserData: () => ({ mutate: vi.fn(), isPending: false, isError: false }),
  appErrorCode: () => undefined,
}));

const preferences = () => useUiStore.getState().preferences;

function open(tab?: string) {
  render(<SettingsDialog />, { wrapper: TooltipProvider });
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));
  if (tab) fireEvent.mouseDown(screen.getByRole("tab", { name: tab }), { button: 0 });
  return screen.getByRole("dialog");
}

describe("SettingsDialog", () => {
  beforeEach(async () => {
    saved.length = 0;
    useUiStore.setState({ settingsTab: null });
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate({ ...DEFAULT_PREFERENCES, experience_profile: "reader" });
  });

  describe("appearance", () => {
    it("chooses a theme from the theme card", () => {
      open();
      const card = screen.getByRole("button", { name: "Colour theme" });
      expect(card).toHaveTextContent("Same as the system");
      expect(screen.queryByRole("radio", { name: "Nord" })).not.toBeInTheDocument();
      fireEvent.click(card);
      expect(screen.getByRole("radio", { name: "Sepia" })).toBeInTheDocument();
      fireEvent.click(screen.getByRole("radio", { name: "Nord" }));
      expect(preferences()["ui.theme"]).toBe("nord");
      // Choosing closes the list.
      expect(screen.queryByRole("radio", { name: "Sepia" })).not.toBeInTheDocument();
      expect(card).toHaveTextContent("Nord");
    });

    it("chooses the Arabic and translation typefaces", () => {
      open();
      const arabic = screen.getByRole("region", { name: "Arabic text" });
      fireEvent.click(within(arabic).getByRole("button", { name: "Typeface" }));
      fireEvent.click(within(arabic).getByRole("radio", { name: /Scheherazade New/ }));
      const translation = screen.getByRole("region", { name: "Translations" });
      fireEvent.click(within(translation).getByRole("button", { name: "Typeface" }));
      fireEvent.click(within(translation).getByRole("radio", { name: "Literata" }));
      expect(preferences()["typography.arabic_font"]).toBe("scheherazade-new");
      expect(preferences()["typography.translation_font"]).toBe("literata");
      fireEvent.click(screen.getByRole("button", { name: "Reset text settings" }));
      expect(preferences()["typography.arabic_font"]).toBe("amiri-quran");
      expect(preferences()["typography.translation_font"]).toBe("system");
    });

    it("adjusts Arabic and translation text separately", () => {
      open();
      const arabic = screen.getByRole("region", { name: "Arabic text" });
      fireEvent.change(within(arabic).getByRole("slider", { name: "Font size" }), {
        target: { value: "34" },
      });
      const translation = screen.getByRole("region", { name: "Translations" });
      fireEvent.change(within(translation).getByRole("slider", { name: "Line spacing" }), {
        target: { value: "2" },
      });
      expect(preferences()["typography.arabic"]).toEqual({ size: 34, line_height: 2.2 });
      expect(preferences()["typography.translation"]).toEqual({ size: 16, line_height: 2 });
    });

    it("keeps Arabic line spacing at 2.0 or more", () => {
      open();
      const arabic = screen.getByRole("region", { name: "Arabic text" });
      expect(within(arabic).getByRole("slider", { name: "Line spacing" })).toHaveAttribute(
        "min",
        "2",
      );
    });
  });

  describe("reading", () => {
    it("leaves what the mode decides to the mode switch", () => {
      const dialog = open("Reading");
      expect(within(dialog).queryByText(/profile/i)).not.toBeInTheDocument();
      expect(within(dialog).queryByRole("checkbox", { name: /side panel/ })).toBeNull();
      expect(within(dialog).queryByRole("checkbox", { name: /search mode chips/ })).toBeNull();
      expect(within(dialog).queryByText("Verse actions")).not.toBeInTheDocument();
    });

    it("shows technical terms the chosen way", () => {
      open("Reading");
      expect(screen.getByText("dictionary form")).toBeInTheDocument();
      fireEvent.click(screen.getByRole("button", { name: "Technical terms" }));
      fireEvent.click(screen.getByRole("radio", { name: /in brackets/ }));
      expect(screen.getByText("dictionary form (lemma)")).toBeInTheDocument();
    });

    it("chooses the citation style", () => {
      open("Reading");
      fireEvent.click(screen.getByRole("button", { name: "Citation style" }));
      fireEvent.click(screen.getByRole("radio", { name: "APA" }));
      expect(preferences()["reader.citation_style"]).toBe("apa");
    });

    it("lets translations be chosen here too", () => {
      open("Reading");
      fireEvent.click(screen.getByRole("checkbox", { name: /Türkçe deneme/ }));
      expect(preferences()["reader.translations"]).toEqual(["en.pickthall", "tr.test"]);
    });
  });

  it("switches the interface language", () => {
    open("Language");
    fireEvent.click(screen.getByRole("button", { name: "Language" }));
    fireEvent.click(screen.getByRole("radio", { name: "Türkçe" }));
    expect(preferences()["ui.language"]).toBe("tr");
  });
});

describe("useApplyTypography", () => {
  it("sets the reader typography variables on <html>", () => {
    renderHook(() => {
      useApplyTypography(
        { size: 32, line_height: 2.5 },
        { size: 18, line_height: 1.8 },
        "scheherazade-new",
        "literata",
      );
    });
    const style = document.documentElement.style;
    expect(style.getPropertyValue("--arabic-size")).toBe("32px");
    expect(style.getPropertyValue("--arabic-line-height")).toBe("2.5");
    expect(style.getPropertyValue("--translation-size")).toBe("18px");
    expect(style.getPropertyValue("--translation-line-height")).toBe("1.8");
    expect(style.getPropertyValue("--arabic-font")).toBe('"Scheherazade New", serif');
    expect(style.getPropertyValue("--translation-font")).toBe('"Literata", serif');
  });
});
