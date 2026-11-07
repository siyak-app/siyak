import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { AiStatus, FoundVerses, RelatedVerse, SemanticStatus } from "@/lib/bindings";
import type { FindVariables } from "@/lib/queries";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { FindView } from "./FindView";

const navigated: unknown[] = [];

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => (to: unknown) => {
    navigated.push(to);
    return Promise.resolve();
  },
}));

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({ cancelQueries: () => Promise.resolve() }),
}));

vi.mock("@/features/ai/LlmPicker", () => ({
  LlmPicker: () => <p>answer model picker</p>,
}));

vi.mock("@/features/ai/EmbeddingPicker", () => ({
  EmbeddingPicker: () => <p>search model picker</p>,
}));

vi.mock("@/features/related/SemanticRequired", () => ({
  SemanticRequired: () => <p>needs the semantic pack</p>,
}));

vi.mock("@/features/related/RelatedVerseList", () => ({
  RelatedVerseList: ({ verses }: { verses: RelatedVerse[] }) => (
    <ul>
      {verses.map((verse) => (
        <li key={`${String(verse.surah)}:${String(verse.ayah)}`}>
          {verse.surah}:{verse.ayah}
        </li>
      ))}
    </ul>
  ),
}));

const READY: AiStatus = {
  state: "ready",
  provider: "anthropic",
  chat_model: "claude-sonnet-5-5",
  translation_model: "claude-sonnet-5-5",
};

let ai: AiStatus;
let semantic: SemanticStatus;
let found: FoundVerses | undefined;
const requests: FindVariables[] = [];

vi.mock("@/lib/queries", () => ({
  queryKeys: { findVerses: (variables: unknown) => ["find-verses", variables] },
  useAiStatus: () => ({ data: ai }),
  useSemanticStatus: () => ({ data: semantic }),
  useFindVerses: (variables: FindVariables) => {
    requests.push(variables);
    return { data: found, isError: false, isFetching: false, refetch: vi.fn() };
  },
}));

function verse(surah: number, ayah: number): RelatedVerse {
  return { surah, ayah } as RelatedVerse;
}

describe("FindView", () => {
  beforeEach(async () => {
    ai = READY;
    semantic = { state: "ready", pack: "vectors.bge-m3", provider: null };
    found = { search_query: "patience in hardship", verses: [verse(2, 153)], cancelled: false };
    navigated.length = 0;
    requests.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("searches the question and lists the verses with the search sentence", () => {
    const { unmount } = render(<FindView q="" />);
    expect(screen.getByText("search model picker")).toBeInTheDocument();
    expect(screen.getByText("answer model picker")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Topic or question"), {
      target: { value: "  being patient  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Find" }));
    expect(navigated).toEqual([{ to: "/find", search: { q: "being patient" } }]);
    unmount();

    render(<FindView q="being patient" />);
    expect(requests.at(-1)).toMatchObject({ query: "being patient", rewrite: true });
    expect(screen.getByText("Searched for: patience in hardship")).toBeInTheDocument();
    expect(screen.getByText("2:153")).toBeInTheDocument();
  });

  it("searches the words as typed when the rewrite is off", () => {
    found = { search_query: null, verses: [verse(2, 153), verse(3, 200)], cancelled: false };
    render(<FindView q="sabır" />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Improve the query with AI" }));
    expect(useUiStore.getState().preferences["find.rewrite"]).toBe(false);
    expect(screen.queryByText("answer model picker")).not.toBeInTheDocument();
    expect(requests.at(-1)).toMatchObject({ query: "sabır", rewrite: false });
    expect(screen.getByText("2 verses found.")).toBeInTheDocument();
  });

  it("searches without the rewrite while AI is not set up, and says why", () => {
    ai = { state: "no_model", provider: "anthropic" };
    render(<FindView q="sabır" />);
    expect(screen.getByText("Choose a provider and an answer model above.")).toBeInTheDocument();
    expect(requests.at(-1)).toMatchObject({ rewrite: false });
  });

  it("asks for a search model before searching", () => {
    semantic = { state: "no_vector_pack" };
    render(<FindView q="sabır" />);
    expect(screen.getByText("needs the semantic pack")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Find" })).toBeDisabled();
    expect(requests).toEqual([]);
  });
});
