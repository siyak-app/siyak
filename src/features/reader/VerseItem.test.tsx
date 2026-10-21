import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { VerseWithTranslations } from "@/lib/bindings";

import { VerseItem } from "./VerseItem";

/** Counts renders of VerseItem: it calls useTranslation once per render. */
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
  translations: [{ pack_id: "en.pickthall", lang: "en", text: "In the name of Allah" }],
};
const DIRECTIONS = { "en.pickthall": "ltr" } as const;

describe("VerseItem", () => {
  it("does not re-render when its parent re-renders with the same props", () => {
    const Parent = ({ tick }: { tick: number }) => (
      <div data-tick={tick}>
        <VerseItem verse={VERSE} directions={DIRECTIONS} />
      </div>
    );
    const { rerender } = render(<Parent tick={1} />);
    rerender(<Parent tick={2} />);
    rerender(<Parent tick={3} />);
    expect(renders.count).toBe(1);
  });
});
