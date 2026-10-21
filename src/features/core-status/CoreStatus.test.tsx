import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { CoreStatus } from "@/lib/bindings";

import { CoreRefreshBanner } from "./CoreStatus";

let status: CoreStatus = { core_version: 1, seed: null, refresh_error: null };

vi.mock("@/lib/queries", () => ({
  useCoreStatus: () => ({ data: status }),
}));

describe("CoreRefreshBanner", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("shows nothing when the core refresh succeeded", () => {
    status = { core_version: 2, seed: null, refresh_error: null };
    const { container } = render(<CoreRefreshBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  it("reports a failed refresh until dismissed", () => {
    status = {
      core_version: 1,
      seed: null,
      refresh_error: { code: "core_refresh_failed", message: "FOREIGN KEY constraint failed" },
    };
    render(<CoreRefreshBanner />);
    expect(screen.getByRole("alert")).toHaveTextContent(/previous data is still in use/);
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
