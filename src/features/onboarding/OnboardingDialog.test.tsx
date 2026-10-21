import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { TranslationInfo } from "@/lib/bindings";
import type { Settings } from "@/lib/settings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { OnboardingDialog } from "./OnboardingDialog";

const PICKTHALL: TranslationInfo = {
  id: "en.pickthall",
  title: "The Meaning of the Glorious Koran",
  titles: { tr: null, en: null },
  lang: "en",
  dir: "ltr",
  bundled: true,
};

const saved: Partial<Settings>[] = [];

vi.mock("@/lib/queries", () => ({
  useTranslations: () => ({ data: [PICKTHALL], isPending: false }),
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

describe("OnboardingDialog", () => {
  beforeEach(async () => {
    saved.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("writes every reader default at once and closes", () => {
    render(<OnboardingDialog />);
    expect(screen.getByRole("dialog")).toHaveTextContent("How would you like to use the app?");
    fireEvent.click(screen.getByRole("button", { name: /Read and understand/ }));

    expect(saved).toHaveLength(1);
    expect(saved[0]).toMatchObject({
      experience_profile: "reader",
      "layout.right_panel": { collapsed: true },
      "reader.translations": ["en.pickthall"],
      "reader.actions": "hover",
      "ui.technical_terms": "tooltip",
      "search.mode_chips": false,
      "reader.citation_action": false,
      "word.click": "card",
      "word.underline": "none",
    });
    expect(useUiStore.getState().preferences.experience_profile).toBe("reader");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("writes the researcher defaults", () => {
    render(<OnboardingDialog />);
    fireEvent.click(screen.getByRole("button", { name: /Do research/ }));
    expect(saved[0]).toMatchObject({
      experience_profile: "researcher",
      "layout.right_panel": { collapsed: false, tab: "word" },
      "reader.actions": "labeled",
      "ui.technical_terms": "inline",
    });
    expect(useUiStore.getState().preferences["reader.actions"]).toBe("labeled");
  });

  it("is not shown once a profile has been chosen", () => {
    useUiStore.getState().setPreference("experience_profile", "researcher");
    render(<OnboardingDialog />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("cannot be dismissed with Escape", () => {
    render(<OnboardingDialog />);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
