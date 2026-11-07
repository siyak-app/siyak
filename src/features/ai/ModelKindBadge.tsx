import { useTranslation } from "react-i18next";

/** What a model does: writes text (LLM) or finds verses by meaning (embedding). */
export type ModelKind = "llm" | "embedding";

/** A small label saying whether a model or key is for an LLM or for embeddings. */
export function ModelKindBadge({ kind }: { kind: ModelKind }) {
  const { t } = useTranslation();
  return (
    <span
      title={t(`models.kind_${kind}_hint`)}
      className="inline-flex shrink-0 items-center rounded-full border px-1.5 py-px text-[0.7rem] leading-4 font-normal text-muted-foreground"
    >
      {t(`models.kind_${kind}`)}
    </span>
  );
}
