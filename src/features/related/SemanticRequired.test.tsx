import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import { useUiStore } from "@/stores/ui";

import { SemanticRequired } from "./SemanticRequired";

vi.mock("@/lib/queries", () => ({
  useCatalog: () => ({ data: undefined }),
}));

describe("SemanticRequired", () => {
  beforeEach(async () => {
    useUiStore.setState({ settingsTab: null, packsDialogOpen: false });
    await i18n.changeLanguage("en");
  });

  it("leads a missing key to the Providers settings", () => {
    render(<SemanticRequired status={{ state: "key_missing", pack: "v", provider: "voyage" }} />);
    expect(screen.getByText(/add your Voyage AI API key/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Open Packs" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    expect(useUiStore.getState().settingsTab).toBe("providers");
  });

  it("explains an unusable server address", () => {
    render(<SemanticRequired status={{ state: "base_url_invalid", pack: "v" }} />);
    expect(screen.getByText(/The server address cannot be used/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open settings" })).toBeInTheDocument();
  });

  it("leads missing packs to the Packs dialog", () => {
    render(<SemanticRequired status={{ state: "no_vector_pack" }} />);
    fireEvent.click(screen.getByRole("button", { name: "Open Packs" }));
    expect(useUiStore.getState().packsDialogOpen).toBe(true);
    expect(screen.queryByRole("button", { name: "Open settings" })).not.toBeInTheDocument();
  });
});
