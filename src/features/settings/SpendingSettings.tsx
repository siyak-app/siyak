import { type SyntheticEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatCost } from "@/features/ai/formatCost";
import type { UsagePeriod, UsageSummary } from "@/lib/bindings";
import { useSetAiSetting, useUsageSummary } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { RadioField } from "./fields";

/**
 * Settings → AI → Spending: what chat and tafsir summaries cost this month
 * and in total, from the provider-reported token counts (spec §9.4), and the
 * optional monthly limit, past which AI actions ask before they run.
 */
export function SpendingSettings() {
  const { t } = useTranslation();
  const [period, setPeriod] = useState<UsagePeriod>("month");
  const summary = useUsageSummary(period);
  return (
    <section className="grid gap-3" aria-labelledby="spending-title">
      <h3 id="spending-title" className="font-medium">
        {t("spending.title")}
      </h3>
      <RadioField<UsagePeriod>
        legend={t("spending.period")}
        value={period}
        options={[
          { value: "month", label: t("spending.month") },
          { value: "total", label: t("spending.total") },
        ]}
        onChange={setPeriod}
      />
      {summary.isError && <ErrorMessage error={summary.error} />}
      {summary.data && <Summary summary={summary.data} />}
      <MonthlyLimit />
    </section>
  );
}

function Summary({ summary }: { summary: UsageSummary }) {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage;
  if (summary.rows.length === 0) {
    return <p className="text-muted-foreground">{t("spending.none")}</p>;
  }
  const unpriced = summary.rows.reduce((sum, row) => sum + row.unpriced, 0);
  return (
    <div className="grid gap-2">
      <p>
        {t("spending.known_total", { cost: formatCost(summary.total_usd ?? 0, language) })}
        {unpriced > 0 && (
          <span className="text-muted-foreground">
            {" "}
            {t("spending.unpriced", { count: unpriced })}
          </span>
        )}
      </p>
      <table className="w-full text-xs">
        <thead className="text-start text-muted-foreground">
          <tr>
            <th className="py-1 text-start font-normal">{t("spending.feature")}</th>
            <th className="py-1 text-start font-normal">{t("spending.model")}</th>
            <th className="py-1 text-end font-normal">{t("spending.requests")}</th>
            <th className="py-1 text-end font-normal">{t("spending.tokens")}</th>
            <th className="py-1 text-end font-normal">{t("spending.cost")}</th>
          </tr>
        </thead>
        <tbody>
          {summary.rows.map((row) => (
            <tr key={`${row.feature}/${row.provider}/${row.model}`} className="border-t">
              <td className="py-1">{t(`spending.feature_${row.feature}`)}</td>
              <td className="py-1 break-all">{row.model}</td>
              <td className="py-1 text-end tabular-nums">{row.requests}</td>
              <td className="py-1 text-end tabular-nums">
                {new Intl.NumberFormat(language).format(row.input_tokens + row.output_tokens)}
              </td>
              <td className="py-1 text-end tabular-nums">
                {row.cost_usd === null ? t("ai.cost_unknown") : formatCost(row.cost_usd, language)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function MonthlyLimit() {
  const { t } = useTranslation();
  const id = useId();
  const hintId = useId();
  const stored = useUiStore((state) => state.preferences["ai.monthly_limit_usd"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const save = useSetAiSetting();
  const [text, setText] = useState(stored === null ? "" : String(stored));
  const parsed = text.trim() === "" ? null : Number(text.replace(",", "."));
  const valid = parsed === null || (Number.isFinite(parsed) && parsed >= 0);
  const changed = valid && parsed !== stored;

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    if (!valid) return;
    setPreference("ai.monthly_limit_usd", parsed);
    save.mutate({ key: "ai.monthly_limit_usd", value: parsed });
  };

  return (
    <form className="grid gap-1.5" onSubmit={submit}>
      <label htmlFor={id} className="font-medium">
        {t("spending.limit")}
      </label>
      <div className="flex gap-2">
        <Input
          id={id}
          inputMode="decimal"
          autoComplete="off"
          placeholder={t("spending.limit_placeholder")}
          aria-describedby={hintId}
          aria-invalid={!valid}
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
        />
        <Button type="submit" variant="outline" size="sm" disabled={!changed || save.isPending}>
          {t("settings.key_save")}
        </Button>
      </div>
      <p id={hintId} className="text-xs text-muted-foreground">
        {t("spending.limit_hint")}
      </p>
      {save.isError && <ErrorMessage error={save.error} />}
    </form>
  );
}
