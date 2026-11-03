import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Catalog, PackView } from "@/lib/bindings";
import { useUiStore } from "@/stores/ui";

import { PacksDialog } from "./PacksDialog";
import { usePackFailures } from "./usePackEvents";

const calls: string[] = [];

function pack(overrides: Partial<PackView>): PackView {
  return {
    id: "x",
    kind: "translation",
    lang: "en",
    title: { tr: null, en: null },
    title_fallback: "X",
    description: null,
    author: null,
    version: "1.0.0",
    size: 1_000_000,
    installed_size: 3_000_000,
    license: "Public domain",
    license_terms: { tr: [], en: [] },
    attribution: null,
    citation: {},
    source_url: null,
    min_app_version: "0.1.0",
    in_manifest: true,
    supported: true,
    installed: false,
    installed_version: null,
    update_available: false,
    incompatible: false,
    bundled: false,
    terms_ack_required: false,
    terms_acknowledged: false,
    replaces: null,
    job: null,
    ...overrides,
  };
}

let catalog: Catalog;

const mutation = (name: string) => ({
  mutate: (variables: unknown, options?: { onSuccess?: () => void }) => {
    const id =
      typeof variables === "object" && variables !== null && "id" in variables
        ? String(variables.id)
        : String(variables);
    calls.push(`${name} ${id}`);
    options?.onSuccess?.();
  },
  mutateAsync: () => Promise.resolve(),
  reset: () => undefined,
  isPending: false,
  isError: false,
  error: null,
  data: undefined,
});

vi.mock("@/lib/queries", () => ({
  useCatalog: () => ({ data: catalog, isPending: false, isError: false }),
  useRefreshCatalog: () => mutation("refresh"),
  useInstallPack: () => mutation("install"),
  useInstallBundle: () => mutation("bundle"),
  useAcknowledgeTerms: () => mutation("ack"),
  useCancelInstall: () => mutation("cancel"),
  useUninstallPack: () => mutation("uninstall"),
  useVacuumContent: () => mutation("vacuum"),
  useOpenPackSource: () => mutation("source"),
  appErrorCode: (error: unknown) => (error ? "network_failed" : undefined),
}));

function open() {
  render(<PacksDialog />);
  fireEvent.click(screen.getByRole("button", { name: "Packs" }));
  return screen.getByRole("dialog");
}

describe("PacksDialog", () => {
  beforeEach(async () => {
    calls.length = 0;
    useUiStore.getState().setPacksDialogOpen(false);
    usePackFailures.setState({ failures: {} });
    await i18n.changeLanguage("en");
    catalog = {
      source: "remote",
      fetched_at: null,
      offline: false,
      error: null,
      bundles: [],
      packs: [
        pack({ id: "en.core", title_fallback: "Pickthall", installed: true, bundled: true }),
        pack({
          id: "ar.jalalayn",
          kind: "tafsir",
          title: { tr: "Celâleyn", en: "Jalalayn" },
          title_fallback: "Jalalayn",
          terms_ack_required: true,
          license_terms: { tr: ["koşul"], en: ["Cite the source"] },
        }),
        pack({ id: "tr.old", title_fallback: "Old", installed: true, installed_version: "1.0.0" }),
        pack({
          id: "tr.busy",
          title_fallback: "Busy",
          job: { state: "running", phase: "download", done: 500_000, total: 1_000_000 },
        }),
      ],
    };
  });

  it("groups packs by type and asks for the terms before installing", () => {
    const dialog = open();
    expect(within(dialog).getByText("Translations")).toBeInTheDocument();
    expect(within(dialog).getByText("Tafsirs")).toBeInTheDocument();
    const jalalayn = within(dialog).getByText("Jalalayn").closest("li");
    if (!jalalayn) throw new Error("row not found");
    expect(jalalayn).toHaveTextContent("Cite the source");
    const install = within(jalalayn).getByRole("button", { name: "Install" });
    expect(install).toBeDisabled();
    fireEvent.click(within(jalalayn).getByRole("checkbox"));
    fireEvent.click(install);
    expect(calls).toEqual(["ack ar.jalalayn", "install ar.jalalayn"]);
  });

  it("never offers to remove a built-in pack", () => {
    const dialog = open();
    const core = within(dialog).getByText("Pickthall").closest("li");
    if (!core) throw new Error("row not found");
    expect(core).toHaveTextContent("Built in");
    expect(within(core).queryByRole("button", { name: "Remove" })).not.toBeInTheDocument();
  });

  it("shows progress with a cancel button", () => {
    const dialog = open();
    const busy = within(dialog).getByText("Busy").closest("li");
    if (!busy) throw new Error("row not found");
    expect(within(busy).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
    expect(busy).toHaveTextContent("Downloading 500 kB of 1 MB");
    fireEvent.click(within(busy).getByRole("button", { name: "Cancel" }));
    expect(calls).toEqual(["cancel tr.busy"]);
  });

  it("suggests compacting the database after a removal", () => {
    const dialog = open();
    const old = within(dialog).getByText("Old").closest("li");
    if (!old) throw new Error("row not found");
    fireEvent.click(within(old).getByRole("button", { name: "Remove" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Compact database" }));
    expect(calls).toEqual(["uninstall tr.old", "vacuum undefined"]);
  });

  it("shows the last failure of a pack", () => {
    usePackFailures.setState({ failures: { "tr.old": "pack_hash_mismatch" } });
    const dialog = open();
    expect(within(dialog).getByRole("alert")).toHaveTextContent("damaged or has been changed");
  });

  it("lists installed packs with their licenses under Sources", () => {
    const dialog = open();
    fireEvent.mouseDown(within(dialog).getByRole("tab", { name: "Sources" }));
    const panel = within(dialog).getByRole("tabpanel");
    expect(panel).toHaveTextContent("Pickthall");
    expect(panel).toHaveTextContent("Old");
    expect(panel).not.toHaveTextContent("Jalalayn");
  });
});
