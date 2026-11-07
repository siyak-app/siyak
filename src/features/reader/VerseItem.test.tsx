import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { VerseWithTranslations } from "@/lib/bindings";

import { VerseItem } from "./VerseItem";

/** Counts renders of VerseItem and its actions: each calls useTranslation once per render. */
const renders = vi.hoisted(() => ({ count: 0 }));

vi.mock("react-i18next", () => ({
  useTranslation: () => {
    renders.count += 1;
    return { t: (key: string) => key };
  },
}));

const VERSE: VerseWithTranslations = {
  id: 1,
  surah: 1,
  ayah: 1,
  juz: 1,
  text_uthmani: "بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ",
  words: [],
  translations: [{ pack_id: "en.pickthall", lang: "en", text: "In the name of Allah" }],
};
const PACKS = { "en.pickthall": { dir: "ltr", title: "Pickthall" } } as const;

describe("VerseItem", () => {
  it("does not re-render when its parent re-renders with the same props", () => {
    const Parent = ({ tick }: { tick: number }) => (
      <div data-tick={tick}>
        <VerseItem verse={VERSE} packs={PACKS} surahName="Al-Faatiha" />
      </div>
    );
    const { rerender } = render(<Parent tick={1} />, { wrapper: TooltipProvider });
    const initial = renders.count;
    rerender(<Parent tick={2} />);
    rerender(<Parent tick={3} />);
    expect(renders.count).toBe(initial);
  });

  it("hides a closed card's translation from assistive technology and opens it on demand", () => {
    const onToggle = vi.fn();
    const { rerender } = render(
      <VerseItem
        verse={VERSE}
        packs={PACKS}
        surahName="Al-Faatiha"
        variant="card"
        onToggle={onToggle}
      />,
      { wrapper: TooltipProvider },
    );
    const toggle = screen.getByRole("button", { name: "reader.show_translation" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText("In the name of Allah").closest("[inert]")).not.toBeNull();
    fireEvent.click(toggle);
    expect(onToggle).toHaveBeenCalledWith(VERSE);
    rerender(
      <VerseItem
        verse={VERSE}
        packs={PACKS}
        surahName="Al-Faatiha"
        variant="card"
        expanded
        onToggle={onToggle}
      />,
    );
    expect(screen.getByRole("button", { name: "reader.hide_translation" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(screen.getByText("In the name of Allah").closest("[inert]")).toBeNull();
  });
});
