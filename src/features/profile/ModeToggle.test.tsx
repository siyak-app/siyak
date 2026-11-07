import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { TranslationInfo } from "@/lib/bindings";
import type { Settings } from "@/lib/settings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { ModeToggle } from "./ModeToggle";

const PACKS: TranslationInfo[] = ["en.pickthall", "en.sahih", "tr.diyanet"].map((id) => ({
  id,
  title: id,
  titles: { tr: null, en: null },
  lang: id.split(".")[0] ?? null,
  dir: "ltr",
  bundled: false,
}));

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
}));

const preferences = () => useUiStore.getState().preferences;

describe("ModeToggle", () => {
  beforeEach(async () => {
    saved.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate({
      ...DEFAULT_PREFERENCES,
      experience_profile: "reader",
      "reader.translations": ["tr.diyanet"],
    });
  });

  it("switches to research mode and keeps the chosen translations", () => {
    render(<ModeToggle />);
    expect(screen.getByRole("radio", { name: "Reading" })).toBeChecked();
    fireEvent.click(screen.getByRole("radio", { name: "Research" }));
    expect(saved).toHaveLength(1);
    expect(preferences()).toMatchObject({
      experience_profile: "researcher",
      "reader.actions": "labeled",
      "ui.technical_terms": "inline",
      "reader.translations": ["tr.diyanet"],
    });
    expect(screen.getByRole("radio", { name: "Research" })).toBeChecked();
  });

  it("moves between the modes with the arrow keys", () => {
    render(<ModeToggle />);
    fireEvent.keyDown(screen.getByRole("radio", { name: "Reading" }), { key: "ArrowRight" });
    expect(preferences().experience_profile).toBe("researcher");
    fireEvent.keyDown(screen.getByRole("radio", { name: "Research" }), { key: "ArrowLeft" });
    expect(preferences().experience_profile).toBe("reader");
  });

  it("does nothing when the current mode is chosen again", () => {
    render(<ModeToggle />);
    fireEvent.click(screen.getByRole("radio", { name: "Reading" }));
    expect(saved).toEqual([]);
  });
});
