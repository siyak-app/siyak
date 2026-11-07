import { describe, expect, it } from "vitest";

import {
  DEFAULT_SETTINGS,
  parseSetting,
  parseSettings,
  SETTING_KEYS,
  serializeSetting,
  type Settings,
} from "./settings";

const SAMPLE: Settings = {
  "ui.language": "tr",
  "ui.theme": "nord",
  "layout.left_pane": { size: 300, collapsed: true },
  "layout.right_panel": { size: 320, collapsed: false, tab: "word" },
  experience_profile: "researcher",
  "reader.translations": ["tr.diyanet", "en.pickthall"],
  "tafsir.active": "ar.jalalayn",
  "compare.columns": ["en.pickthall", "ar.jalalayn"],
  "reader.actions": "labeled",
  "word.click": "panel",
  "word.underline": "dotted",
  "ui.technical_terms": "inline",
  "search.mode_chips": true,
  "reader.citation_action": true,
  "reader.citation_style": "apa",
  "typography.arabic": { size: 32, line_height: 2.4 },
  "typography.translation": { size: 18, line_height: 1.8 },
  "typography.arabic_font": "scheherazade-new",
  "typography.translation_font": "literata",
  "reader.last_position": { surah: 2, ayah: 255 },
  "semantic.vector_pack": "vectors.bge-m3",
  "semantic.base_url": "http://localhost:11434/v1",
  "ai.provider": "anthropic",
  "ai.chat_model": "claude-sonnet",
  "ai.translation_model": null,
  "ai.arabic_sources": { "openai_compatible/qwen3": true },
  "ai.monthly_limit_usd": 5,
  "chat.tafsirs": ["ar.jalalayn"],
  "find.rewrite": false,
  "audio.reciter": "audio.alafasy",
  "audio.autoplay": false,
  "audio.repeat": "surah",
  "audio.follow": false,
};

describe("settings", () => {
  it("round-trips every setting through its stored JSON text", () => {
    const stored = Object.fromEntries(
      SETTING_KEYS.map((key) => [key, serializeSetting(key, SAMPLE[key])]),
    );
    expect(parseSettings(stored)).toEqual(SAMPLE);
  });

  it("uses defaults when nothing is stored", () => {
    expect(parseSettings({})).toEqual(DEFAULT_SETTINGS);
    expect(DEFAULT_SETTINGS.experience_profile).toBeNull();
  });

  it("falls back to the default for malformed or out-of-range values", () => {
    expect(parseSetting("reader.translations", '["en.pickthall", 3]')).toEqual(["en.pickthall"]);
    expect(parseSetting("typography.arabic", '{"size":28,"line_height":1.2}')).toEqual(
      DEFAULT_SETTINGS["typography.arabic"],
    );
    expect(parseSetting("reader.last_position", '{"surah":0,"ayah":1}')).toBeNull();
    expect(parseSetting("experience_profile", '"expert"')).toBeNull();
    expect(parseSetting("audio.repeat", '"forever"')).toBe("off");
    expect(parseSetting("audio.reciter", '""')).toBeNull();
    expect(parseSetting("search.mode_chips", '"yes"')).toBe(false);
    expect(parseSetting("ui.theme", "{")).toBe("system");
    expect(parseSetting("ui.theme", '"monokai"')).toBe("system");
    expect(parseSetting("typography.arabic_font", '"comic-sans"')).toBe("amiri-quran");
    expect(parseSetting("typography.translation_font", "3")).toBe("system");
    expect(parseSetting("ai.provider", '"voyage"')).toBeNull();
    expect(parseSetting("ai.arabic_sources", '{"a/b":"yes"}')).toEqual({});
  });

  it("keeps an empty translation selection (Arabic only)", () => {
    expect(parseSetting("reader.translations", "[]")).toEqual([]);
  });
});
