import { describe, expect, it } from "vitest";

import { DEFAULT_PREFERENCES, preferencesFromSettings } from "./ui";

describe("preferencesFromSettings", () => {
  it("uses defaults when nothing is stored", () => {
    expect(preferencesFromSettings({})).toEqual(DEFAULT_PREFERENCES);
  });

  it("reads valid stored values", () => {
    const preferences = preferencesFromSettings({
      "ui.language": '"tr"',
      "layout.left_pane": '{"size":320,"collapsed":true}',
      "layout.right_panel": '{"size":300,"collapsed":false,"tab":"notes"}',
    });
    expect(preferences["ui.language"]).toBe("tr");
    expect(preferences["layout.left_pane"]).toEqual({ size: 320, collapsed: true });
    expect(preferences["layout.right_panel"].tab).toBe("notes");
  });

  it("falls back to defaults for malformed or unexpected values", () => {
    const preferences = preferencesFromSettings({
      "ui.language": "not json",
      "ui.theme": '"purple"',
      "layout.left_pane": '{"size":-5,"collapsed":false}',
      "layout.right_panel": '{"size":300,"collapsed":false,"tab":"unknown"}',
    });
    expect(preferences).toEqual(DEFAULT_PREFERENCES);
  });
});
