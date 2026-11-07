import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import i18n from "@/i18n";
import type { UsageSummary } from "@/lib/bindings";
import { DEFAULT_PREFERENCES, useUiStore } from "@/stores/ui";

import { SpendingSettings } from "./SpendingSettings";

let summary: UsageSummary;
const periods: string[] = [];
const saved: { key: string; value: unknown }[] = [];

vi.mock("@/lib/queries", () => ({
  useUsageSummary: (period: string) => {
    periods.push(period);
    return { data: summary, isError: false };
  },
  useSetAiSetting: () => ({
    mutate: (value: { key: string; value: unknown }) => saved.push(value),
    isPending: false,
    isError: false,
  }),
  appErrorCode: () => undefined,
}));

describe("SpendingSettings", () => {
  beforeEach(async () => {
    summary = { total_usd: 0, rows: [], monthly_limit_usd: null };
    periods.length = 0;
    saved.length = 0;
    await i18n.changeLanguage("en");
    useUiStore.getState().hydrate(DEFAULT_PREFERENCES);
  });

  it("says when nothing was spent and switches the period", () => {
    render(<SpendingSettings />);
    expect(screen.getByText("Nothing spent yet.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "All time" }));
    expect(periods.at(-1)).toBe("total");
  });

  it("lists spending per feature and model, with unknown prices", () => {
    summary = {
      total_usd: 0.42,
      monthly_limit_usd: null,
      rows: [
        {
          feature: "chat",
          provider: "anthropic",
          model: "claude-sonnet-5-5",
          requests: 3,
          input_tokens: 9000,
          output_tokens: 1000,
          cost_usd: 0.42,
          unpriced: 0,
        },
        {
          feature: "translate",
          provider: "openai_compatible",
          model: "qwen3",
          requests: 2,
          input_tokens: 10,
          output_tokens: 5,
          cost_usd: null,
          unpriced: 2,
        },
      ],
    };
    render(<SpendingSettings />);
    expect(screen.getByText("Known cost: $0.42")).toBeInTheDocument();
    expect(screen.getByText(/2 requests with an unknown price/)).toBeInTheDocument();
    expect(screen.getByText("10,000")).toBeInTheDocument();
    expect(screen.getByText("unknown")).toBeInTheDocument();
  });

  it("saves the monthly limit and refuses a negative one", () => {
    render(<SpendingSettings />);
    const input = screen.getByLabelText("Monthly limit (USD)");
    const save = screen.getByRole("button", { name: "Save" });
    fireEvent.change(input, { target: { value: "-1" } });
    expect(save).toBeDisabled();
    fireEvent.change(input, { target: { value: "2,5" } });
    fireEvent.click(save);
    expect(saved).toEqual([{ key: "ai.monthly_limit_usd", value: 2.5 }]);
    expect(useUiStore.getState().preferences["ai.monthly_limit_usd"]).toBe(2.5);
  });
});
