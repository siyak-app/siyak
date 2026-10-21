import { describe, expect, it } from "vitest";

import { fold, foldName } from "./fold";

describe("fold", () => {
  it("removes accents, punctuation and case", () => {
    expect(fold("Âl-i İmrân")).toBe("aliimran");
    expect(fold("Ğâşiye")).toBe("gasiye");
    expect(fold("Fussılet")).toBe("fussilet");
  });

  it("removes harakat and unifies Arabic letter variants", () => {
    expect(fold("البَقَرَة")).toBe("البقره");
    expect(fold("الإخلاص")).toBe(fold("الاخلاص"));
    expect(fold("آل عمران")).toBe("العمران");
  });
});

describe("foldName", () => {
  it("lets different transliterations meet", () => {
    for (const name of ["Al-Baqara", "Al-Baqarah", "Baqarah", "baqara"]) {
      expect(foldName(name)).toBe("baqara");
    }
    expect(foldName("Al-Faatiha")).toBe(foldName("Al-Fatihah"));
    expect(foldName("Yaseen")).toBe(foldName("Yasin"));
    expect(foldName("At-Tawba")).toBe(foldName("Tawbah"));
    expect(foldName("Ash-Shu'araa")).toBe("shuara");
  });

  it("keeps names that start like an article", () => {
    expect(foldName("Aal-i-Imraan")).toBe("alimran");
    expect(foldName("Ali 'Imran")).toBe("alimran");
  });
});
