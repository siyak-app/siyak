import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { ImportReport } from "@/lib/bindings";

import { BackupSettings } from "./BackupSettings";

const exportData = vi.fn();
const importData = vi.fn();
let saved: string | null;
let report: ImportReport | null;

vi.mock("@/lib/queries", () => ({
  appErrorCode: () => undefined,
  useExportUserData: () => ({ mutate: exportData, isPending: false, isError: false, data: saved }),
  useImportUserData: () => ({ mutate: importData, isPending: false, isError: false, data: report }),
}));

describe("BackupSettings", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    exportData.mockClear();
    importData.mockClear();
    saved = null;
    report = null;
  });

  it("saves a backup and says where", () => {
    saved = "/tmp/siyak.json";
    render(<BackupSettings />);
    fireEvent.click(screen.getByRole("button", { name: "Save a backup…" }));
    expect(exportData).toHaveBeenCalledOnce();
    expect(screen.getByRole("status")).toHaveTextContent("/tmp/siyak.json");
  });

  it("merges at once", () => {
    render(<BackupSettings />);
    fireEvent.click(screen.getByRole("button", { name: "Add to what is here…" }));
    expect(importData).toHaveBeenCalledWith("merge");
  });

  it("replaces only after confirming", () => {
    render(<BackupSettings />);
    fireEvent.click(screen.getByRole("button", { name: "Replace everything…" }));
    expect(importData).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Yes, replace my data" }));
    expect(importData).toHaveBeenCalledWith("replace");
  });

  it("reports what was added and what was left out", () => {
    report = {
      bookmarks: 2,
      notes: 3,
      collections: 1,
      items: 4,
      tafsir_translations: 0,
      skipped: 1,
    };
    render(<BackupSettings />);
    expect(screen.getByRole("status")).toHaveTextContent("2 bookmarks, 3 notes, 1 collections");
    expect(screen.getByText(/1 entry was left out/)).toBeInTheDocument();
  });
});
