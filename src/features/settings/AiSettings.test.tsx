import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { AiStatus, ModelInfo } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { AiSettings } from "./AiSettings";

let status: AiStatus;
let models: { data?: ModelInfo[]; isError: boolean; isSuccess: boolean; error?: Error };
const saved: { key: string; value: unknown }[] = [];
const modelRequests: (string | null)[] = [];

vi.mock("@/features/ai/EmbeddingPicker", () => ({
  EmbeddingPicker: () => <p>search model picker</p>,
}));

vi.mock("@/lib/queries", () => ({
  useAiStatus: () => ({ data: status }),
  useModels: (provider: string | null, enabled: boolean) => {
    if (enabled) modelRequests.push(provider);
    return { ...models, isPending: false };
  },
  useTafsirs: () => ({
    data: ["ar.jalalayn", "ar.ibn-kathir", "ar.baghawi", "ar.qurtubi"].map((id) => ({
      id,
      title: id,
      titles: { tr: null, en: null },
    })),
  }),
  useUsageSummary: () => ({ data: { total_usd: 0, rows: [], monthly_limit_usd: null } }),
  useSetAiSetting: () => ({
    mutate: (value: { key: string; value: unknown }) => saved.push(value),
    isError: false,
  }),
  appErrorCode: () => undefined,
}));

function submit(input: HTMLElement) {
  const form = input.closest("form");
  if (!form) throw new Error("no form");
  fireEvent.submit(form);
}

describe("AiSettings", () => {
  beforeEach(async () => {
    status = { state: "no_provider" };
    models = { data: [], isError: false, isSuccess: true };
    saved.length = 0;
    modelRequests.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("starts with AI off and chooses a provider, clearing its models", () => {
    render(<AiSettings />);
    const provider = screen.getByRole("button", { name: "Provider" });
    expect(provider).toHaveTextContent("None (AI off)");
    fireEvent.click(provider);
    expect(screen.getByRole("radio", { name: "None (AI off)" })).toBeChecked();
    expect(screen.getByRole("heading", { name: "Answer model" })).toBeInTheDocument();
    expect(screen.getByText("LLM")).toBeInTheDocument();
    expect(screen.getByText("search model picker")).toBeInTheDocument();
    expect(screen.queryByText("Chat model")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "Anthropic" }));
    expect(saved).toEqual([
      { key: "ai.provider", value: "anthropic" },
      { key: "ai.chat_model", value: null },
      { key: "ai.translation_model", value: null },
    ]);
    expect(useUiStore.getState().preferences["ai.provider"]).toBe("anthropic");
  });

  it("sends the user to the Providers tab when the key is missing", () => {
    useUiStore.getState().setPreference("ai.provider", "gemini");
    status = { state: "key_missing", provider: "gemini" };
    render(<AiSettings />);
    expect(screen.getByText(/add your Google Gemini API key/)).toBeInTheDocument();
    expect(modelRequests).toEqual([]);
    fireEvent.click(screen.getByRole("button", { name: "Add it on the Providers tab" }));
    expect(useUiStore.getState().settingsTab).toBe("providers");
  });

  it("lets the model be typed when the list cannot be loaded", () => {
    useUiStore.getState().setPreference("ai.provider", "openai_compatible");
    status = { state: "no_model", provider: "openai_compatible" };
    models = { isError: true, isSuccess: false, error: new Error("offline") };
    render(<AiSettings />);
    expect(modelRequests).toContain("openai_compatible");
    const input = screen.getByLabelText("Chat model");
    fireEvent.change(input, { target: { value: " qwen3 " } });
    submit(input);
    expect(saved).toEqual([{ key: "ai.chat_model", value: "qwen3" }]);
  });

  it("keeps Arabic sources off for a local server unless turned on", () => {
    useUiStore.getState().setPreferences({
      "ai.provider": "openai_compatible",
      "ai.chat_model": "qwen3",
    });
    status = {
      state: "ready",
      provider: "openai_compatible",
      chat_model: "qwen3",
      translation_model: "qwen3",
    };
    render(<AiSettings />);
    const arabic = screen.getByRole("checkbox", {
      name: "The chat model can read classical Arabic",
    });
    expect(arabic).not.toBeChecked();
    fireEvent.click(arabic);
    expect(saved).toEqual([
      { key: "ai.arabic_sources", value: { "openai_compatible/qwen3": true } },
    ]);
  });

  it("chooses at most three chat tafsirs", () => {
    useUiStore.getState().setPreference("ai.provider", "anthropic");
    status = {
      state: "ready",
      provider: "anthropic",
      chat_model: "m",
      translation_model: "m",
    };
    render(<AiSettings />);
    expect(screen.getByRole("checkbox", { name: "ar.jalalayn" })).toBeChecked();
    fireEvent.click(screen.getByRole("checkbox", { name: "ar.baghawi" }));
    expect(saved.at(-1)).toEqual({
      key: "chat.tafsirs",
      value: ["ar.jalalayn", "ar.ibn-kathir", "ar.baghawi"],
    });
    expect(screen.getByRole("checkbox", { name: "ar.qurtubi" })).toBeDisabled();
  });
});
