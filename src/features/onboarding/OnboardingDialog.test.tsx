import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Catalog, PackView, TranslationInfo } from "@/lib/bindings";
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
const calls: string[] = [];

const QAC = {
  id: "morphology.qac",
  kind: "morphology",
  title: { tr: "Kuran Korpusu", en: "Quranic Corpus" },
  title_fallback: "Quranic Corpus",
  version: "1.0.0",
  size: 4_200_000,
  license: "QAC terms of use",
  license_terms: { tr: ["Değiştirilemez"], en: ["No modification"] },
  installed: false,
  terms_ack_required: true,
  terms_acknowledged: false,
  replaces: null,
  requires: [],
  job: null,
  embedding: null,
} as unknown as PackView;

const CATALOG = {
  packs: [QAC],
  bundles: [
    {
      id: "recommended",
      title: { tr: "Önerilen paketler", en: "Recommended packs" },
      description: null,
      packs: ["morphology.qac"],
      missing: ["morphology.qac"],
      size: 4_200_000,
      installed_size: 30_000_000,
    },
  ],
} as unknown as Catalog;

let catalog: Catalog;

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
  useCatalog: () => ({ data: catalog, isPending: false, isError: false }),
  useAcknowledgeTerms: () => ({
    mutateAsync: ({ id }: { id: string }) => {
      calls.push(`ack ${id}`);
      return Promise.resolve();
    },
    isPending: false,
    error: null,
  }),
  useInstallBundle: () => ({
    mutate: ({ id }: { id: string }) => {
      calls.push(`bundle ${id}`);
    },
    isPending: false,
    error: null,
  }),
  useCancelInstall: () => ({ mutate: () => undefined, isPending: false }),
  appErrorCode: () => undefined,
}));

describe("OnboardingDialog", () => {
  beforeEach(async () => {
    saved.length = 0;
    calls.length = 0;
    catalog = CATALOG;
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
    expect(screen.getByRole("dialog")).toHaveTextContent("Recommended packs");
    fireEvent.click(screen.getByRole("button", { name: "Not now" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(calls).toEqual([]);
  });

  it("installs the recommended packs once their terms are accepted", async () => {
    render(<OnboardingDialog />);
    fireEvent.click(screen.getByRole("button", { name: /Read and understand/ }));
    expect(screen.getByRole("dialog")).not.toHaveTextContent("No modification");
    fireEvent.click(screen.getByRole("button", { name: "Download (4.2 MB)" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("No modification");
    const install = screen.getByRole("button", { name: "Accept and download" });
    expect(install).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /I have read these terms/ }));
    fireEvent.click(install);
    await waitFor(() => {
      expect(calls).toEqual(["ack morphology.qac", "bundle recommended"]);
    });
  });

  it("shows the license and terms before download even when they need no acknowledgement", () => {
    catalog = {
      ...CATALOG,
      packs: [{ ...QAC, terms_ack_required: false }],
    };
    render(<OnboardingDialog />);
    fireEvent.click(screen.getByRole("button", { name: /Read and understand/ }));
    fireEvent.click(screen.getByRole("button", { name: "Download (4.2 MB)" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("QAC terms of use");
    expect(dialog).toHaveTextContent("No modification");
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Download (4.2 MB)" })).toBeEnabled();
  });

  it("heads the packs step with its own question", () => {
    render(<OnboardingDialog />);
    fireEvent.click(screen.getByRole("button", { name: /Read and understand/ }));
    expect(screen.getByRole("heading", { name: "Add more content?" })).toBeInTheDocument();
    expect(screen.getByRole("dialog")).not.toHaveTextContent("How would you like to use the app?");
  });

  it("offers to continue once the recommended packs are installed", () => {
    const [bundle] = CATALOG.bundles;
    if (!bundle) throw new Error("no bundle");
    catalog = {
      packs: [{ ...QAC, installed: true }],
      bundles: [{ ...bundle, missing: [] }],
    } as unknown as Catalog;
    render(<OnboardingDialog />);
    fireEvent.click(screen.getByRole("button", { name: /Read and understand/ }));
    expect(screen.queryByRole("button", { name: "Not now" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
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
