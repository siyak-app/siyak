import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { ApiKeyStatus, SemanticStatus } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { ProviderSettings } from "./ProviderSettings";

let keys: ApiKeyStatus[];
let status: SemanticStatus;
const savedKeys: { provider: string; key: string }[] = [];
const removed: string[] = [];
const settings: { key: string; value: string | null }[] = [];

const succeed =
  <V,>(log: V[]) =>
  (value: V, options?: { onSuccess?: () => void }) => {
    log.push(value);
    options?.onSuccess?.();
  };

vi.mock("@/lib/queries", () => ({
  useApiKeys: () => ({ data: keys, isError: false }),
  useSetApiKey: () => ({ mutate: succeed(savedKeys), isPending: false, isError: false }),
  useDeleteApiKey: () => ({ mutate: succeed(removed), isPending: false, isError: false }),
  useSetSemanticSetting: () => ({ mutate: succeed(settings), isPending: false, isError: false }),
  useSemanticStatus: () => ({ data: status }),
  appErrorCode: () => undefined,
}));

function section(name: RegExp) {
  const found = screen.getByRole("heading", { name }).closest("section");
  if (!found) throw new Error(`no section ${String(name)}`);
  return found;
}

function submit(input: HTMLElement) {
  const form = input.closest("form");
  if (!form) throw new Error("no form");
  fireEvent.submit(form);
}

describe("ProviderSettings", () => {
  beforeEach(async () => {
    keys = [
      { provider: "openai", saved: true },
      { provider: "voyage", saved: false },
      { provider: "gemini", saved: false },
      { provider: "openai_compatible", saved: false },
    ];
    status = { state: "ready", pack: "vectors.bge-m3", provider: null };
    savedKeys.length = 0;
    removed.length = 0;
    settings.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("shows which keys are saved, never the keys", () => {
    render(<ProviderSettings />);
    expect(within(section(/^OpenAI$/)).getByText("Saved")).toBeInTheDocument();
    expect(within(section(/^Voyage AI$/)).getByText("Not saved")).toBeInTheDocument();
    expect(within(section(/^OpenAI$/)).getByLabelText("OpenAI API key")).toHaveValue("");
    expect(
      within(section(/^OpenAI-compatible server/)).getByText("(key optional)"),
    ).toBeInTheDocument();
    expect(screen.getByText("Create a separate key for this app.")).toBeInTheDocument();
  });

  it("marks what each key is used for", () => {
    render(<ProviderSettings />);
    const anthropic = within(section(/^Anthropic$/));
    expect(anthropic.getByText("LLM")).toBeInTheDocument();
    expect(anthropic.queryByText("Embedding")).not.toBeInTheDocument();
    const voyage = within(section(/^Voyage AI$/));
    expect(voyage.getByText("Embedding")).toBeInTheDocument();
    expect(voyage.queryByText("LLM")).not.toBeInTheDocument();
    const openai = within(section(/^OpenAI$/));
    expect(openai.getByText("LLM")).toBeInTheDocument();
    expect(openai.getByText("Embedding")).toBeInTheDocument();
  });

  it("saves a key and clears the field", () => {
    render(<ProviderSettings />);
    const voyage = section(/^Voyage AI$/);
    const input = within(voyage).getByLabelText("Voyage AI API key");
    const save = within(voyage).getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    fireEvent.change(input, { target: { value: "  pa-123 " } });
    fireEvent.click(save);
    expect(savedKeys).toEqual([{ provider: "voyage", key: "  pa-123 " }]);
    expect(input).toHaveValue("");
  });

  it("removes only saved keys", () => {
    render(<ProviderSettings />);
    expect(
      within(section(/^Voyage AI$/)).queryByRole("button", { name: "Remove" }),
    ).not.toBeInTheDocument();
    fireEvent.click(within(section(/^OpenAI$/)).getByRole("button", { name: "Remove" }));
    expect(removed).toEqual(["openai"]);
  });

  it("saves the server address, or clears it", () => {
    render(<ProviderSettings />);
    const input = screen.getByLabelText("OpenAI-compatible server address");
    fireEvent.change(input, { target: { value: " http://localhost:11434/v1 " } });
    submit(input);
    expect(settings).toEqual([{ key: "semantic.base_url", value: "http://localhost:11434/v1" }]);
    expect(useUiStore.getState().preferences["semantic.base_url"]).toBe(
      "http://localhost:11434/v1",
    );

    fireEvent.change(input, { target: { value: "  " } });
    submit(input);
    expect(settings.at(-1)).toEqual({ key: "semantic.base_url", value: null });
  });

  it("says when the saved address cannot be used", () => {
    status = { state: "base_url_invalid", pack: "vectors.ollama" };
    useUiStore.getState().setPreference("semantic.base_url", "http://192.168.1.5:11434/v1");
    render(<ProviderSettings />);
    expect(screen.getByText(/The server address cannot be used/)).toBeInTheDocument();
  });
});
