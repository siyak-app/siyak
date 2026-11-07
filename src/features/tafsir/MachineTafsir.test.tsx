import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type {
  AiStatus,
  CostEstimate,
  TafsirInfo,
  TafsirPassage,
  TafsirTranslation,
} from "@/lib/bindings";
import type { TafsirJobVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { MachineTafsir } from "./MachineTafsir";

/** A stand-in for Tauri's channel: tests push pieces through `onmessage`. */
interface FakeChannel {
  onmessage: (piece: string) => void;
}

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (piece: string) => void = () => undefined;
  },
}));

const TAFSIR: TafsirInfo = {
  id: "ar.jalalayn",
  title: "Tafsir al-Jalalayn",
  titles: { tr: null, en: null },
  author: null,
  lang: "ar",
  dir: "rtl",
  attribution: null,
  has_introduction: false,
};

const PASSAGE: TafsirPassage = {
  id: 7,
  verse_from: { surah: 2, ayah: 153 },
  verse_to: { surah: 2, ayah: 153 },
  paragraphs: [
    { idx: 0, text: "الفقرة الأولى", part: null, page: null },
    { idx: 1, text: "الفقرة الثانية", part: null, page: null },
  ],
};

function translation(partial: Partial<TafsirTranslation>): TafsirTranslation {
  return {
    paragraph_idx: null,
    mode: "summary",
    target_lang: "en",
    text: "",
    provider: "anthropic",
    model: "claude-sonnet-5-5",
    created_at: "2026-11-07T10:00:00Z",
    ...partial,
  };
}

let status: AiStatus;
let saved: TafsirTranslation[];
let overLimit: boolean;
const runs: TafsirJobVariables[] = [];
const refreshed: string[] = [];
let finishRun: () => void;

vi.mock("@/features/ai/LlmPicker", () => ({
  LlmPicker: ({ role }: { role: string }) => <p>{role} model picker</p>,
}));

vi.mock("@/lib/queries", () => ({
  useAiStatus: () => ({ data: status }),
  useTafsirTranslations: () => ({ data: saved, isError: false }),
  useTafsirEstimate: (variables: TafsirJobVariables): { data: CostEstimate } => ({
    data: {
      input_tokens: 100,
      output_tokens: 50,
      cost_usd: variables.mode === "summary" ? 0.003 : null,
      provider: "anthropic",
      model: "claude-sonnet-5-5",
      over_limit: overLimit,
    },
  }),
  useTafsirTranslated: () => (packId: string) => {
    refreshed.push(packId);
    return Promise.resolve();
  },
  translateTafsir: (variables: TafsirJobVariables, _id: string, channel: FakeChannel) => {
    runs.push(variables);
    channel.onmessage("First ");
    channel.onmessage("paragraph.");
    return new Promise((resolve) => {
      finishRun = () => {
        resolve({ translation: null, finish: "complete" });
      };
    });
  },
  cancelGeneration: () => Promise.resolve(null),
  appErrorCode: () => undefined,
}));

describe("MachineTafsir", () => {
  beforeEach(async () => {
    status = {
      state: "ready",
      provider: "anthropic",
      chat_model: "claude-sonnet-5-5",
      translation_model: "claude-sonnet-5-5",
    };
    saved = [];
    overLimit = false;
    runs.length = 0;
    refreshed.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("offers nothing for a tafsir in the interface language", () => {
    render(<MachineTafsir tafsir={{ ...TAFSIR, lang: "en", dir: "ltr" }} passage={PASSAGE} />);
    expect(screen.queryByRole("button", { name: /Summarize/ })).not.toBeInTheDocument();
    expect(screen.getByText("الفقرة الأولى")).toBeInTheDocument();
  });

  it("shows the model that writes and the estimated cost on each action", () => {
    render(<MachineTafsir tafsir={TAFSIR} passage={PASSAGE} />);
    expect(screen.getByText("translation model picker")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Summarize (≈ 0.3 ¢)" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Translate (cost unknown)" })).toBeInTheDocument();
  });

  it("shows saved text next to the original with its label, and highlights the original", () => {
    saved = [
      translation({ text: "A short summary." }),
      translation({ mode: "full", paragraph_idx: 0, text: "The first paragraph." }),
    ];
    render(<MachineTafsir tafsir={TAFSIR} passage={PASSAGE} />);
    expect(screen.getByText("A short summary.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Summarize/ })).not.toBeInTheDocument();
    // The next untranslated paragraph is offered as "Translate more".
    expect(screen.getByRole("button", { name: /Translate more/ })).toBeInTheDocument();
    const label = screen.getByRole("button", {
      name: "Machine translation · Anthropic/claude-sonnet-5-5 · Nov 7, 2026",
    });
    fireEvent.click(label);
    expect(screen.getByText("الفقرة الأولى")).toHaveAttribute("data-highlighted", "true");
    expect(screen.getByText("The first paragraph.").closest("[lang]")).toHaveAttribute(
      "lang",
      "en",
    );
  });

  it("streams a translation below its paragraph and refreshes when done", async () => {
    saved = [translation({ mode: "full", paragraph_idx: 0, text: "Done before." })];
    render(<MachineTafsir tafsir={TAFSIR} passage={PASSAGE} />);
    fireEvent.click(screen.getByRole("button", { name: /Translate more/ }));
    expect(runs).toEqual([
      { packId: "ar.jalalayn", tafsirId: 7, paragraphIdx: 1, mode: "full", targetLang: "en" },
    ]);
    expect(await screen.findByText("First paragraph.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();
    await act(async () => {
      finishRun();
      await Promise.resolve();
    });
    expect(refreshed).toEqual(["ar.jalalayn"]);
    expect(screen.queryByRole("button", { name: "Stop" })).not.toBeInTheDocument();
  });

  it("asks before passing the monthly limit", () => {
    overLimit = true;
    render(<MachineTafsir tafsir={TAFSIR} passage={PASSAGE} />);
    fireEvent.click(screen.getByRole("button", { name: /Summarize/ }));
    const confirm = screen.getByRole("alertdialog", { name: "Summarize" });
    expect(runs).toEqual([]);
    fireEvent.click(within(confirm).getByRole("button", { name: "Continue" }));
    expect(runs).toHaveLength(1);
    expect(runs[0]?.mode).toBe("summary");
  });

  it("points to the AI settings when AI is not set up", () => {
    status = { state: "no_provider" };
    render(<MachineTafsir tafsir={TAFSIR} passage={PASSAGE} />);
    fireEvent.click(screen.getByRole("button", { name: "Summarize" }));
    fireEvent.click(screen.getByRole("button", { name: "Set up AI" }));
    expect(useUiStore.getState().settingsTab).toBe("ai");
    expect(runs).toEqual([]);
  });
});
