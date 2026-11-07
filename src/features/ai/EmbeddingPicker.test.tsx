import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { PackView } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { EmbeddingPicker } from "./EmbeddingPicker";

let packs: Partial<PackView>[];
const settings: { key: string; value: string | null }[] = [];

vi.mock("@/lib/queries", () => ({
  useCatalog: () => ({ data: { packs }, isPending: false, isError: false }),
  useSetSemanticSetting: () => ({
    mutate: (
      value: { key: string; value: string | null },
      options?: { onSuccess?: () => void },
    ) => {
      settings.push(value);
      options?.onSuccess?.();
    },
    isError: false,
  }),
  appErrorCode: () => undefined,
}));

function vectorPack(id: string, overrides: Partial<PackView> = {}): Partial<PackView> {
  return {
    id,
    kind: "vectors",
    installed: true,
    supported: true,
    incompatible: false,
    title: { tr: null, en: id },
    title_fallback: id,
    embedding: null,
    ...overrides,
  };
}

const LOCAL = vectorPack("vectors.bge-m3", { embedding: { provider: null, model: "bge-m3" } });
const VOYAGE = vectorPack("vectors.voyage-4-large", {
  embedding: { provider: "voyage", model: "voyage-4-large" },
});

describe("EmbeddingPicker", () => {
  beforeEach(async () => {
    settings.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("sends the user to Packs when no search model is installed", () => {
    packs = [{ ...VOYAGE, installed: false }];
    render(<EmbeddingPicker />);
    expect(screen.getByText(/No search model is installed yet/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Download a search model" }));
    expect(useUiStore.getState().packsDialogOpen).toBe(true);
  });

  it("names each model by who embeds the queries and saves the choice", () => {
    packs = [LOCAL, VOYAGE];
    render(<EmbeddingPicker />);
    expect(screen.getByText("Embedding")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "On this computer · bge-m3" })).toBeChecked();
    fireEvent.click(screen.getByRole("radio", { name: "Voyage AI · voyage-4-large" }));
    expect(settings).toEqual([{ key: "semantic.vector_pack", value: "vectors.voyage-4-large" }]);
    expect(useUiStore.getState().preferences["semantic.vector_pack"]).toBe(
      "vectors.voyage-4-large",
    );
    expect(
      screen.queryByRole("button", { name: "Download another search model" }),
    ).not.toBeInTheDocument();
  });

  it("offers another download while more models can be installed", () => {
    packs = [LOCAL, { ...VOYAGE, installed: false }];
    render(<EmbeddingPicker compact />);
    expect(screen.getByLabelText("Search model")).toHaveTextContent("On this computer · bge-m3");
    fireEvent.click(screen.getByRole("button", { name: "Download another search model" }));
    expect(useUiStore.getState().packsDialogOpen).toBe(true);
  });
});
