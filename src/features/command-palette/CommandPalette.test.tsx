import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { Surah } from "@/lib/bindings";

import { CommandPalette } from "./CommandPalette";

const surah = (number: number, name_tr: string, name_en: string, ayah_count: number): Surah => ({
  number,
  name_tr,
  name_en,
  name_ar: "سورة",
  ayah_count,
  revelation_type: "medinan",
  bismillah: null,
});

const SURAHS = [
  surah(1, "Fâtiha", "Al-Faatiha", 7),
  surah(2, "Bakara", "Al-Baqara", 286),
  surah(113, "Felak", "Al-Falaq", 5),
];

const navigate = vi.hoisted(() => vi.fn());

vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));
vi.mock("@/lib/queries", () => ({ useSurahs: () => ({ data: SURAHS }) }));

function openWithShortcut() {
  render(<CommandPalette />);
  // jsdom's user agent is not a Mac's, so the shortcut is Ctrl+K.
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
  return screen.getByRole("combobox", { name: "Go to" });
}

describe("CommandPalette", () => {
  beforeEach(async () => {
    navigate.mockClear();
    await i18n.changeLanguage("en");
  });

  it("opens with Ctrl+K and goes to a verse", () => {
    const input = openWithShortcut();
    fireEvent.change(input, { target: { value: "Baqarah 255" } });
    fireEvent.click(screen.getByRole("option", { name: "Go to Al-Baqara 2:255" }));
    expect(navigate).toHaveBeenCalledWith({
      to: "/read/$surah/$ayah",
      params: { surah: 2, ayah: 255 },
    });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("goes to the target with Enter", () => {
    const input = openWithShortcut();
    fireEvent.change(input, { target: { value: "2:255" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(navigate).toHaveBeenCalledWith({
      to: "/read/$surah/$ayah",
      params: { surah: 2, ayah: 255 },
    });
  });

  it("lists matching surahs while the name is ambiguous", () => {
    const input = openWithShortcut();
    fireEvent.change(input, { target: { value: "F" } });
    fireEvent.click(screen.getByRole("option", { name: "Go to 113. Al-Falaq" }));
    expect(navigate).toHaveBeenCalledWith({ to: "/read/$surah", params: { surah: 113 } });
  });

  it("explains why an input leads nowhere", () => {
    const input = openWithShortcut();
    fireEvent.change(input, { target: { value: "2:300" } });
    expect(screen.getByText("That surah has no such verse.")).toBeInTheDocument();
    fireEvent.change(input, { target: { value: "120" } });
    expect(screen.getByText("There are 114 surahs.")).toBeInTheDocument();
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
  });

  it("uses the Turkish names in Turkish", async () => {
    await i18n.changeLanguage("tr");
    render(<CommandPalette />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "bakara 255" } });
    expect(screen.getByRole("option", { name: "Bakara 2:255 ayetine git" })).toBeInTheDocument();
  });

  it("can be opened from its button", () => {
    render(<CommandPalette />);
    fireEvent.click(screen.getByRole("button", { name: /Go to/ }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
