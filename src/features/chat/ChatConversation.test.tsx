import { act, fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { AiStatus, ChatSession, CostEstimate, SemanticStatus } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { ChatConversation } from "./ChatConversation";

interface FakeChannel {
  onmessage: (piece: string) => void;
}

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (piece: string) => void = () => undefined;
  },
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    params,
  }: {
    children: ReactNode;
    params?: { surah: number; ayah: number };
  }) => (
    <a href={params ? `#/read/${String(params.surah)}/${String(params.ayah)}` : "#"}>{children}</a>
  ),
}));

vi.mock("@/features/related/SemanticRequired", () => ({
  SemanticRequired: () => <p>needs the semantic pack</p>,
}));

const READY: AiStatus = {
  state: "ready",
  provider: "anthropic",
  chat_model: "claude-sonnet-5-5",
  translation_model: "claude-sonnet-5-5",
};

let ai: AiStatus;
let semantic: SemanticStatus;
let session: ChatSession | undefined;
let estimate: CostEstimate | undefined;
const sent: { sessionId: number | null; message: string; language: string }[] = [];
const notes: unknown[] = [];
let finish: (reply: unknown) => void;

vi.mock("@/lib/queries", () => ({
  useAiStatus: () => ({ data: ai }),
  useSemanticStatus: () => ({ data: semantic }),
  useChatSession: () => ({ data: session, isError: false }),
  useChatEstimate: () => ({ data: estimate }),
  useChatAnswered: () => () => Promise.resolve(),
  useUpsertNote: () => ({
    isPending: false,
    mutate: (note: unknown, options: { onSuccess: (n: { id: number }) => void }) => {
      notes.push(note);
      options.onSuccess({ id: 42 });
    },
  }),
  sendChat: (
    variables: { sessionId: number | null; message: string; language: string },
    _id: string,
    channel: FakeChannel,
  ) => {
    sent.push(variables);
    channel.onmessage("Patience ");
    channel.onmessage("[2:153].");
    return new Promise((resolve) => {
      finish = resolve;
    });
  },
  cancelGeneration: () => Promise.resolve(null),
  appErrorCode: () => undefined,
}));

const SESSION: ChatSession = {
  session: {
    id: 5,
    title: "Patience",
    provider: "anthropic",
    model: "claude-sonnet-5-5",
    created_at: "2026-11-07T10:00:00Z",
    updated_at: "2026-11-07T10:00:00Z",
  },
  messages: [
    {
      id: 1,
      role: "user",
      content: "What is patience?",
      cited: [],
      sources: null,
      created_at: "2026-11-07T10:00:00Z",
    },
    {
      id: 2,
      role: "assistant",
      content: "Allah is with the patient [2:153].",
      cited: [{ verse_id: 160, verse: { surah: 2, ayah: 153 } }],
      sources: {
        verses: [{ verse_id: 160, verse: { surah: 2, ayah: 153 }, score: 0.9 }],
        tafsirs: [
          {
            pack_id: "ar.jalalayn",
            title: "Tafsir al-Jalalayn",
            tafsir_id: 9,
            verse_from: { surah: 2, ayah: 153 },
            verse_to: { surah: 2, ayah: 154 },
            paragraphs: [0],
            complete: false,
          },
        ],
        notices: [{ kind: "arabic_tafsirs_skipped" }],
        search_query: "Patience and prayer as a help",
      },
      created_at: "2026-11-07T10:00:05Z",
    },
  ],
};

function type(text: string) {
  fireEvent.change(screen.getByLabelText("Your question"), { target: { value: text } });
}

describe("ChatConversation", () => {
  beforeEach(async () => {
    ai = READY;
    semantic = { state: "ready", pack: "vectors.bge-m3", provider: null };
    session = undefined;
    estimate = undefined;
    sent.length = 0;
    notes.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("shows messages with clickable citations, sources and the notice", () => {
    session = SESSION;
    render(<ChatConversation sessionId={5} onSession={vi.fn()} />);
    expect(screen.getByText("What is patience?")).toBeInTheDocument();
    const citation = screen.getByRole("link", { name: "[2:153]" });
    expect(citation).toHaveAttribute("href", "#/read/2/153");
    expect(screen.getByText(/only translations were used/)).toBeInTheDocument();
    expect(screen.getByText("Sources used (2)")).toBeInTheDocument();
    expect(screen.getByText(/Tafsir al-Jalalayn · 2:153–154/)).toBeInTheDocument();
    expect(screen.getByText("Searched for: Patience and prayer as a help")).toBeInTheDocument();
    expect(screen.getByText(/not a religious ruling/)).toBeInTheDocument();
  });

  it("saves an answer as a labelled note and opens the collection picker", () => {
    session = SESSION;
    render(<ChatConversation sessionId={5} onSession={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Add to collection" }));
    expect(notes).toEqual([
      {
        id: null,
        verse_from: 160,
        verse_to: 160,
        body: "**AI answer** · Anthropic/claude-sonnet-5-5 · Nov 7, 2026\n\nAllah is with the patient [2:153].",
      },
    ]);
    expect(useUiStore.getState().collectionPicker).toEqual({ kind: "note", id: 42 });
  });

  it("streams an answer and hands over the new session", async () => {
    const onSession = vi.fn();
    render(<ChatConversation sessionId={null} onSession={onSession} />);
    type("  What is patience?  ");
    fireEvent.keyDown(screen.getByLabelText("Your question"), { key: "Enter" });
    expect(sent).toEqual([{ sessionId: null, message: "What is patience?", language: "en" }]);
    expect(await screen.findByText("What is patience?")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "[2:153]" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();
    await act(async () => {
      finish({ session_id: 7, question: {}, answer: {}, finish: "complete" });
      await Promise.resolve();
    });
    expect(onSession).toHaveBeenCalledWith(7);
  });

  it("asks before passing the monthly limit and shows the estimate", () => {
    estimate = {
      input_tokens: 8000,
      output_tokens: 800,
      cost_usd: 0.024,
      provider: "anthropic",
      model: "claude-sonnet-5-5",
      over_limit: true,
    };
    render(<ChatConversation sessionId={null} onSession={vi.fn()} />);
    type("Patience?");
    expect(screen.getByText("Estimated cost: ≈ $0.024")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(sent).toEqual([]);
    expect(screen.getByRole("alert")).toHaveTextContent(/monthly spending limit/);
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(sent).toHaveLength(1);
  });

  it("says what is missing and keeps the composer off", () => {
    ai = { state: "no_provider" };
    const { unmount } = render(<ChatConversation sessionId={null} onSession={vi.fn()} />);
    expect(screen.getByText("Chat needs an AI provider and model.")).toBeInTheDocument();
    expect(screen.getByLabelText("Your question")).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    expect(useUiStore.getState().settingsTab).toBe("ai");
    unmount();

    ai = READY;
    semantic = { state: "no_vector_pack" };
    render(<ChatConversation sessionId={null} onSession={vi.fn()} />);
    expect(screen.getByText("needs the semantic pack")).toBeInTheDocument();
    expect(screen.getByLabelText("Your question")).toBeDisabled();
  });
});
