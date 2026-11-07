import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Catalog, PackView } from "@/lib/bindings";
import { useUiStore } from "@/stores/ui";

import { PacksDialog } from "./PacksDialog";
import { BundleCard } from "./BundleCard";
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
    requires: [],
    job: null,
    embedding: null,
    style: null,
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
  mutateAsync: (variables: unknown) => {
    calls.push(`${name} ${typeof variables === "string" ? variables : "?"}`);
    return Promise.resolve();
  },
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

/** Switches the Packs dialog to the section named `name`. */
function show(dialog: HTMLElement, name: RegExp) {
  fireEvent.mouseDown(within(dialog).getByRole("tab", { name }));
  return within(dialog).getByRole("tabpanel");
}

function row(container: HTMLElement, title: string) {
  const item = within(container).getByText(title).closest("li");
  if (!item) throw new Error(`row ${title} not found`);
  return item;
}

/** Opens the Details of a row. */
function details(item: HTMLElement) {
  fireEvent.click(within(item).getByText("Details"));
  return item;
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
          lang: "ar",
          title: { tr: "Celâleyn", en: "Jalalayn" },
          title_fallback: "Jalalayn",
          terms_ack_required: true,
          license_terms: { tr: ["koşul"], en: ["Cite the source"] },
        }),
        pack({
          id: "tr.old",
          lang: "tr",
          title_fallback: "Old",
          installed: true,
          installed_version: "1.0.0",
        }),
        pack({
          id: "tr.busy",
          lang: "tr",
          title_fallback: "Busy",
          job: { state: "running", phase: "download", done: 500_000, total: 1_000_000 },
        }),
      ],
    };
  });

  it("lists the packs of one type at a time, by language", () => {
    const dialog = open();
    expect(within(dialog).getByRole("tab", { name: /^Tafsirs/ })).toBeInTheDocument();
    expect(within(dialog).getByRole("tab", { name: /^Recitations/ })).toBeInTheDocument();
    expect(within(dialog).queryByRole("tab", { name: /^Search by meaning/ })).toBeNull();
    const panel = within(dialog).getByRole("tabpanel");
    expect(panel).toHaveTextContent("Pickthall");
    expect(panel).not.toHaveTextContent("Jalalayn");
    const headings = within(panel)
      .getAllByRole("heading")
      .map((h) => h.textContent);
    expect(headings).toEqual(["English", "Turkish"]);
  });

  it("asks for the terms before downloading", () => {
    const dialog = open();
    const jalalayn = row(show(dialog, /^Tafsirs/), "Jalalayn");
    expect(jalalayn).not.toHaveTextContent("I have read");
    fireEvent.click(within(jalalayn).getByRole("button", { name: "Download" }));
    expect(jalalayn).toHaveTextContent("Cite the source");
    const accept = within(jalalayn).getByRole("button", { name: "Accept and download" });
    expect(accept).toBeDisabled();
    fireEvent.click(within(jalalayn).getByRole("checkbox"));
    fireEvent.click(accept);
    expect(calls).toEqual(["ack ar.jalalayn", "install ar.jalalayn"]);
  });

  it("shows terms that need no acceptance before downloading", () => {
    catalog.packs.push(
      pack({
        id: "en.terms",
        lang: "en",
        title_fallback: "Termed",
        license_terms: { tr: ["koşul"], en: ["Non-commercial use only"] },
      }),
    );
    const dialog = open();
    const termed = row(dialog, "Termed");
    fireEvent.click(within(termed).getByRole("button", { name: "Download" }));
    expect(calls).toEqual([]);
    expect(termed).toHaveTextContent("Non-commercial use only");
    expect(within(termed).queryByRole("checkbox")).toBeNull();
    fireEvent.click(within(termed).getByRole("button", { name: "Download" }));
    expect(calls).toEqual(["install en.terms"]);
  });

  it("opens on the section another screen asks for", () => {
    useUiStore.getState().setPacksDialogOpen(true, "tafsir");
    render(<PacksDialog />);
    expect(screen.getByRole("tab", { name: /^Tafsirs/ })).toHaveAttribute("aria-selected", "true");
  });

  it("opens on the recommended sets until something is installed", () => {
    catalog.packs = catalog.packs.filter((p) => p.bundled || !p.installed);
    catalog.bundles = [
      {
        id: "recommended",
        title: { tr: "Önerilen", en: "Starter set" },
        description: null,
        packs: ["ar.jalalayn"],
        missing: ["ar.jalalayn"],
        size: 1_000_000,
        installed_size: 2_000_000,
      },
    ];
    const dialog = open();
    expect(within(dialog).getByRole("tabpanel")).toHaveTextContent("Starter set");
  });

  it("never offers to remove a built-in pack", () => {
    const dialog = open();
    const core = details(row(dialog, "Pickthall"));
    expect(core).toHaveTextContent("Built in");
    expect(within(core).queryByRole("button", { name: "Remove" })).not.toBeInTheDocument();
  });

  it("keeps the license and version under Details", () => {
    const dialog = open();
    const old = row(dialog, "Old");
    const summary = within(old).getByText("Details").closest("details");
    expect(summary).not.toHaveAttribute("open");
    expect(summary).toHaveTextContent("License: Public domain");
    expect(summary).toHaveTextContent("Version 1.0.0");
    expect(dialog).not.toHaveTextContent("tr.old");
  });

  it("shows progress with a cancel button", () => {
    const dialog = open();
    const busy = row(dialog, "Busy");
    expect(within(busy).getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
    expect(busy).toHaveTextContent("Downloading 500 kB of 1 MB");
    fireEvent.click(within(busy).getByRole("button", { name: "Cancel" }));
    expect(calls).toEqual(["cancel tr.busy"]);
  });

  it("suggests compacting the database after a removal", async () => {
    const dialog = open();
    const old = details(row(dialog, "Old"));
    fireEvent.click(within(old).getByRole("button", { name: "Remove" }));
    fireEvent.click(await within(dialog).findByRole("button", { name: "Compact database" }));
    expect(calls).toEqual(["uninstall tr.old", "vacuum undefined"]);
  });

  it("searches by title or author and filters installed packs", () => {
    catalog.packs.push(
      pack({
        id: "tr.yazir",
        lang: "tr",
        title_fallback: "Yazır",
        author: "Elmalılı Hamdi Yazır",
      }),
    );
    const dialog = open();
    const panel = within(dialog).getByRole("tabpanel");
    fireEvent.change(within(panel).getByRole("searchbox"), { target: { value: "elmalili" } });
    expect(panel).toHaveTextContent("Yazır");
    expect(panel).not.toHaveTextContent("Pickthall");
    fireEvent.change(within(panel).getByRole("searchbox"), { target: { value: "" } });
    fireEvent.click(within(panel).getByRole("checkbox", { name: "Installed only" }));
    expect(panel).toHaveTextContent("Pickthall");
    expect(panel).toHaveTextContent("Old");
    expect(panel).not.toHaveTextContent("Yazır");
  });

  describe("search by meaning", () => {
    beforeEach(() => {
      catalog.packs.push(
        pack({
          id: "vectors.bge",
          kind: "vectors",
          lang: null,
          title_fallback: "Search by meaning (BGE)",
          size: 24_000_000,
          requires: ["model.bge"],
          embedding: { provider: null, model: "bge-m3" },
        }),
        pack({
          id: "model.bge",
          kind: "model",
          lang: null,
          title_fallback: "BGE model",
          size: 394_000_000,
        }),
      );
    });

    it("shows the model as part of the vector pack", () => {
      const dialog = open();
      const panel = show(dialog, /^Search by meaning/);
      expect(within(panel).getAllByRole("listitem")).toHaveLength(1);
      const vectors = row(panel, "Search by meaning (BGE)");
      expect(vectors).toHaveTextContent("Runs on this computer; no internet needed");
      expect(vectors).toHaveTextContent("24 MB · includes a model of about 394 MB");
      fireEvent.click(within(vectors).getByRole("button", { name: "Download" }));
      expect(calls).toEqual(["install vectors.bge"]);
    });

    it("removes the model with the last pack that needs it", async () => {
      catalog.packs = catalog.packs.map((p) =>
        p.id === "vectors.bge" || p.id === "model.bge"
          ? { ...p, installed: true, installed_version: "1.0.0" }
          : p,
      );
      const dialog = open();
      const vectors = details(row(show(dialog, /^Search by meaning/), "Search by meaning (BGE)"));
      fireEvent.click(within(vectors).getByRole("button", { name: "Remove" }));
      await waitFor(() => {
        expect(calls).toEqual(["uninstall vectors.bge", "uninstall model.bge"]);
      });
    });

    it("names the API a vector pack needs a key for", () => {
      catalog.packs.push(
        pack({
          id: "vectors.voyage",
          kind: "vectors",
          lang: null,
          title_fallback: "Search by meaning (Voyage)",
          embedding: { provider: "voyage", model: "voyage-4-large" },
        }),
      );
      const dialog = open();
      expect(
        row(show(dialog, /^Search by meaning/), "Search by meaning (Voyage)"),
      ).toHaveTextContent("Needs a Voyage AI API key");
    });
  });

  it("shows the recitation style of an audio pack", () => {
    catalog.packs.push(
      pack({
        id: "audio.x",
        kind: "audio",
        lang: null,
        title_fallback: "Reciter",
        style: "mujawwad",
      }),
    );
    const dialog = open();
    expect(row(show(dialog, /^Recitations/), "Reciter")).toHaveTextContent("Mujawwad");
  });

  it("shows the last failure of a pack", () => {
    usePackFailures.setState({ failures: { "tr.old": "pack_hash_mismatch" } });
    const dialog = open();
    expect(within(dialog).getByRole("alert")).toHaveTextContent("damaged or has been changed");
  });

  it("shows why a pack of a bundle failed", () => {
    usePackFailures.setState({ failures: { "ar.jalalayn": "pack_needs_full_data" } });
    render(
      <BundleCard
        catalog={catalog}
        bundle={{
          id: "recommended",
          title: { tr: "Önerilen", en: "Recommended" },
          description: null,
          packs: ["ar.jalalayn"],
          missing: ["ar.jalalayn"],
          size: 1000,
          installed_size: 2000,
        }}
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(/^Jalalayn: .*SIYAK_DEV_DATA=dev/);
  });

  it("lists installed packs with their licenses under Sources", () => {
    const dialog = open();
    const panel = show(dialog, /^Sources/);
    expect(panel).toHaveTextContent("Pickthall");
    expect(panel).toHaveTextContent("Old");
    expect(panel).not.toHaveTextContent("Jalalayn");
  });
});
