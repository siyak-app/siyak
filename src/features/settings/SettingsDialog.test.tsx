import { fireEvent, render, renderHook, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useApplyTypography } from "@/app/preferences";
import { TooltipProvider } from "@/components/ui/tooltip";
import { PROFILE_KEYS } from "@/features/profile/profileDefaults";
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
  useSetSettings: () => ({
    mutate: (values: Partial<Settings>, options: { onSuccess: () => void }) => {
      saved.push(values);
      options.onSuccess();
    },
    isPending: false,
    isError: false,
  }),
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
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate({ ...DEFAULT_PREFERENCES, experience_profile: "reader" });
  });

  describe("appearance", () => {
    it("offers the sepia theme", () => {
      open();
      fireEvent.click(screen.getByRole("radio", { name: "Sepia" }));
      expect(preferences()["ui.theme"]).toBe("sepia");
    });

    it("adjusts Arabic and translation text separately", () => {
      open();
      const arabic = screen.getByRole("group", { name: "Arabic text" });
      fireEvent.change(within(arabic).getByRole("slider", { name: "Font size" }), {
        target: { value: "34" },
      });
      const translation = screen.getByRole("group", { name: "Translations" });
      fireEvent.change(within(translation).getByRole("slider", { name: "Line spacing" }), {
        target: { value: "2" },
      });
      expect(preferences()["typography.arabic"]).toEqual({ size: 34, line_height: 2.2 });
      expect(preferences()["typography.translation"]).toEqual({ size: 16, line_height: 2 });
    });

    it("keeps Arabic line spacing at 2.0 or more", () => {
      open();
      const arabic = screen.getByRole("group", { name: "Arabic text" });
      expect(within(arabic).getByRole("slider", { name: "Line spacing" })).toHaveAttribute(
        "min",
        "2",
      );
    });
  });

  describe("display & behaviour", () => {
    it("disables the settings of features that do not exist yet", () => {
      const dialog = open("Display & behaviour");
      expect(within(dialog).getAllByText("Available in a later version")).toHaveLength(3);
      expect(screen.getByRole("radio", { name: "Shows a small card" })).toBeDisabled();
      expect(screen.getByRole("radio", { name: "Dotted underline" })).toBeDisabled();
      expect(screen.getByRole("checkbox", { name: "Show search mode chips" })).toBeEnabled();
      expect(screen.getByRole("checkbox", { name: /Copy citation/ })).toBeDisabled();
      expect(screen.getByRole("radio", { name: /Always visible/ })).toBeEnabled();
    });

    it("changes a single setting", () => {
      open("Display & behaviour");
      fireEvent.click(screen.getByRole("radio", { name: /Always visible/ }));
      expect(preferences()["reader.actions"]).toBe("labeled");
      const collapsed = preferences()["layout.right_panel"].collapsed;
      fireEvent.click(screen.getByRole("checkbox", { name: "Show the side panel" }));
      expect(preferences()["layout.right_panel"].collapsed).toBe(!collapsed);
    });

    it("shows technical terms the chosen way", () => {
      open("Display & behaviour");
      expect(screen.getByText("dictionary form")).toBeInTheDocument();
      fireEvent.click(screen.getByRole("radio", { name: /in brackets/ }));
      expect(screen.getByText("dictionary form (lemma)")).toBeInTheDocument();
    });

    it("resets every profile setting to the researcher defaults", () => {
      open("Display & behaviour");
      fireEvent.click(screen.getByRole("button", { name: "Reset to “Do research” defaults" }));
      expect(saved).toHaveLength(1);
      expect(Object.keys(saved[0] ?? {}).sort()).toEqual([...PROFILE_KEYS].sort());
      expect(preferences()).toMatchObject({
        experience_profile: "researcher",
        "reader.actions": "labeled",
        "reader.translations": ["en.pickthall", "tr.test"],
        "ui.technical_terms": "inline",
      });
      expect(screen.getByText("Current profile: Do research")).toBeInTheDocument();
    });

    it("lets translations be chosen here too", () => {
      open("Display & behaviour");
      fireEvent.click(screen.getByRole("checkbox", { name: /Türkçe deneme/ }));
      expect(preferences()["reader.translations"]).toEqual(["en.pickthall", "tr.test"]);
    });
  });

  it("switches the interface language", () => {
    open("Language");
    fireEvent.click(screen.getByRole("radio", { name: "Türkçe" }));
    expect(preferences()["ui.language"]).toBe("tr");
  });
});

describe("useApplyTypography", () => {
  it("sets the reader typography variables on <html>", () => {
    renderHook(() => {
      useApplyTypography({ size: 32, line_height: 2.5 }, { size: 18, line_height: 1.8 });
    });
    const style = document.documentElement.style;
    expect(style.getPropertyValue("--arabic-size")).toBe("32px");
    expect(style.getPropertyValue("--arabic-line-height")).toBe("2.5");
    expect(style.getPropertyValue("--translation-size")).toBe("18px");
    expect(style.getPropertyValue("--translation-line-height")).toBe("1.8");
  });
});
