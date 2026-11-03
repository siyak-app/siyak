import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { QueryClass } from "@/lib/bindings";

import { SearchBox } from "./SearchBox";

const navigate = vi.fn();
let result: QueryClass | Error;
let urlQuery: unknown;

vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => ({}) }));
vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigate,
  useSearch: () => ({ q: urlQuery }),
}));
vi.mock("@/lib/queries", () => ({
  classifyQuery: () => (result instanceof Error ? Promise.reject(result) : Promise.resolve(result)),
}));

function submit(value: string) {
  const box = screen.getByRole("searchbox");
  fireEvent.change(box, { target: { value } });
  fireEvent.submit(box);
}

describe("SearchBox", () => {
  beforeEach(async () => {
    navigate.mockClear();
    urlQuery = undefined;
    await i18n.changeLanguage("en");
  });

  it("jumps to a verse reference without searching", async () => {
    result = { kind: "reference", surah: 2, ayah: 255, ayah_to: null };
    render(<SearchBox />);
    submit("2:255");
    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({
        to: "/read/$surah/$ayah",
        params: { surah: 2, ayah: 255 },
      });
    });
  });

  it("opens a whole surah", async () => {
    result = { kind: "reference", surah: 36, ayah: null, ayah_to: null };
    render(<SearchBox />);
    submit("36");
    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: "/read/$surah", params: { surah: 36 } });
    });
  });

  it("shows results for anything else, and if classifying fails", async () => {
    result = { kind: "text" };
    render(<SearchBox />);
    submit(" sabır ");
    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({
        to: "/search",
        search: { q: "sabır", mode: "all" },
      });
    });
    navigate.mockClear();
    result = new Error("boom");
    submit("x");
    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: "/search", search: { q: "x", mode: "all" } });
    });
  });

  it("ignores an empty box and follows the URL's query", () => {
    urlQuery = "merciful";
    const { rerender } = render(<SearchBox />);
    expect(screen.getByRole("searchbox")).toHaveValue("merciful");
    submit("   ");
    expect(navigate).not.toHaveBeenCalled();
    urlQuery = "patience";
    rerender(<SearchBox />);
    expect(screen.getByRole("searchbox")).toHaveValue("patience");
  });
});
