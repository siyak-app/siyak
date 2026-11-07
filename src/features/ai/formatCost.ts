/**
 * A cost in US dollars as the interface shows it (spec §9.2): costs under a
 * cent in cents ("0.3 ¢"), larger ones in dollars. `null` means the model's
 * price is unknown (a local model, or one missing from `pricing.json`).
 */
export function formatCost(usd: number, language: string | undefined): string {
  if (usd > 0 && usd < 0.01) {
    const cents = usd * 100;
    if (cents < 0.1) return "< 0.1 ¢";
    return `${new Intl.NumberFormat(language, { maximumFractionDigits: 1 }).format(cents)} ¢`;
  }
  return new Intl.NumberFormat(language, {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: usd < 1 ? 3 : 2,
  }).format(usd);
}
