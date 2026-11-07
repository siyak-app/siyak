import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { formatCost } from "@/features/ai/formatCost";
import { useStream } from "@/features/ai/useStream";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import type {
  CostEstimate,
  ProviderId,
  TafsirInfo,
  TafsirPassage,
  TafsirTranslation,
  TranslationMode,
} from "@/lib/bindings";
import {
  type TafsirJobVariables,
  translateTafsir,
  useAiStatus,
  useTafsirEstimate,
  useTafsirTranslated,
  useTafsirTranslations,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { TafsirParagraphs } from "./TafsirParagraphs";

/** How long a paragraph stays highlighted after its translation's label is clicked. */
const HIGHLIGHT_MS = 2000;

/** What is being written: the summary, or the translation of a paragraph. */
interface Target {
  mode: TranslationMode;
  idx: number | null;
}

/**
 * A tafsir passage with on-demand machine summaries and translations
 * (spec §9, ADR 0004). Offered only when the tafsir is not in the interface
 * language. Machine text is always shown next to the original, never in its
 * place, and labelled with its provider, model and date; clicking the label
 * highlights the original paragraph.
 */
export function MachineTafsir({ tafsir, passage }: { tafsir: TafsirInfo; passage: TafsirPassage }) {
  const { i18n } = useTranslation();
  const target = i18n.resolvedLanguage === "tr" ? "tr" : "en";
  const offered = tafsir.lang !== null && tafsir.lang.split("-")[0] !== target;
  if (!offered) return <TafsirParagraphs tafsir={tafsir} passage={passage} />;
  return <WithMachineText tafsir={tafsir} passage={passage} target={target} />;
}

function WithMachineText({
  tafsir,
  passage,
  target,
}: {
  tafsir: TafsirInfo;
  passage: TafsirPassage;
  target: string;
}) {
  const { t } = useTranslation();
  const saved = useTafsirTranslations(tafsir.id, passage.id, target);
  const translated = useTafsirTranslated();
  const { streaming, start, stop } = useStream<Target>();
  const [error, setError] = useState<unknown>(null);
  const [highlighted, setHighlighted] = useState<number | null>(null);

  useEffect(() => {
    if (highlighted === null) return;
    const timer = setTimeout(() => {
      setHighlighted(null);
    }, HIGHLIGHT_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [highlighted]);

  const translations = saved.data ?? [];
  const summary = translations.find((item) => item.mode === "summary");
  const full = new Map(
    translations
      .filter((item) => item.mode === "full" && item.paragraph_idx !== null)
      .map((item) => [item.paragraph_idx, item]),
  );
  const next = passage.paragraphs.find((paragraph) => !full.has(paragraph.idx))?.idx ?? null;

  const run = async ({ mode, idx }: Target) => {
    setError(null);
    try {
      await start({ mode, idx }, (requestId, channel) =>
        translateTafsir(
          { packId: tafsir.id, tafsirId: passage.id, paragraphIdx: idx, mode, targetLang: target },
          requestId,
          channel,
        ),
      );
      await translated(tafsir.id, passage.id, target);
    } catch (caught) {
      setError(caught);
    }
  };

  const streamingFor = (mode: TranslationMode, idx: number | null) =>
    streaming?.key.mode === mode && streaming.key.idx === idx ? streaming.text : null;

  const job = (mode: TranslationMode, idx: number | null): TafsirJobVariables => ({
    packId: tafsir.id,
    tafsirId: passage.id,
    paragraphIdx: idx,
    mode,
    targetLang: target,
  });

  return (
    <div className="grid gap-3">
      <Toolbar
        summary={summary ? null : job("summary", null)}
        translate={next === null ? null : job("full", next)}
        more={full.size > 0}
        busy={streaming !== null}
        onRun={(variables) => {
          void run({ mode: variables.mode, idx: variables.paragraphIdx });
        }}
        onStop={stop}
      />
      {error !== null && <ErrorMessage error={error} />}
      {summary ? (
        <MachineText translation={summary} />
      ) : (
        streamingFor("summary", null) !== null && (
          <MachineText streamingText={streamingFor("summary", null) ?? ""} mode="summary" />
        )
      )}
      <TafsirParagraphs
        tafsir={tafsir}
        passage={passage}
        highlighted={highlighted}
        after={(idx) => {
          const done = full.get(idx);
          if (done) {
            return (
              <MachineText
                translation={done}
                onLabelClick={() => {
                  setHighlighted(idx);
                }}
              />
            );
          }
          const text = streamingFor("full", idx);
          return text === null ? null : <MachineText streamingText={text} mode="full" />;
        }}
      />
      {saved.isError && (
        <p className="text-xs text-muted-foreground">{t("tafsir_ai.cache_failed")}</p>
      )}
    </div>
  );
}

function Toolbar({
  summary,
  translate,
  more,
  busy,
  onRun,
  onStop,
}: {
  /** The summary job, if there is no summary yet. */
  summary: TafsirJobVariables | null;
  /** The next paragraph's translation job, if one is left. */
  translate: TafsirJobVariables | null;
  /** Whether some paragraph is translated already ("Translate more"). */
  more: boolean;
  busy: boolean;
  onRun: (variables: TafsirJobVariables) => void;
  onStop: () => void;
}) {
  const { t } = useTranslation();
  const status = useAiStatus();
  const setSettingsTab = useUiStore((state) => state.setSettingsTab);
  const [setup, setSetup] = useState(false);
  const ready = status.data?.state === "ready";

  if (!summary && !translate && !busy) return null;
  if (busy) {
    return (
      <div className="flex items-center gap-2" role="status">
        <span className="text-xs text-muted-foreground">{t("tafsir_ai.writing")}</span>
        <Button variant="outline" size="sm" onClick={onStop}>
          {t("tafsir_ai.stop")}
        </Button>
      </div>
    );
  }
  if (!ready) {
    return (
      <div className="grid gap-1">
        <div className="flex flex-wrap gap-1">
          {summary && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setSetup(true);
              }}
            >
              {t("tafsir_ai.summarize")}
            </Button>
          )}
          {translate && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setSetup(true);
              }}
            >
              {more ? t("tafsir_ai.translate_more") : t("tafsir_ai.translate")}
            </Button>
          )}
        </div>
        {setup && (
          <p className="text-xs text-muted-foreground">
            {t("tafsir_ai.setup")}{" "}
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0 text-xs"
              onClick={() => {
                setSettingsTab("ai");
              }}
            >
              {t("tafsir_ai.open_settings")}
            </Button>
          </p>
        )}
      </div>
    );
  }
  return (
    <div className="flex flex-wrap gap-1">
      {summary && <CostButton variables={summary} label={t("tafsir_ai.summarize")} onRun={onRun} />}
      {translate && (
        <CostButton
          variables={translate}
          label={more ? t("tafsir_ai.translate_more") : t("tafsir_ai.translate")}
          onRun={onRun}
        />
      )}
    </div>
  );
}

/** A button with the estimated cost; past the monthly limit it asks first (spec §9.4). */
function CostButton({
  variables,
  label,
  onRun,
}: {
  variables: TafsirJobVariables;
  label: string;
  onRun: (variables: TafsirJobVariables) => void;
}) {
  const { t } = useTranslation();
  const estimate = useTafsirEstimate(variables, true);
  const [confirming, setConfirming] = useState(false);
  const cost = useCostText(estimate.data);

  if (confirming) {
    return (
      <div className="flex flex-wrap items-center gap-1" role="alertdialog" aria-label={label}>
        <span className="text-xs">{t("tafsir_ai.over_limit")}</span>
        <Button
          size="sm"
          onClick={() => {
            setConfirming(false);
            onRun(variables);
          }}
        >
          {t("tafsir_ai.continue")}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setConfirming(false);
          }}
        >
          {t("tafsir_ai.cancel")}
        </Button>
      </div>
    );
  }
  return (
    <Button
      variant="outline"
      size="sm"
      onClick={() => {
        if (estimate.data?.over_limit) setConfirming(true);
        else onRun(variables);
      }}
    >
      {cost ? `${label} (${cost})` : label}
    </Button>
  );
}

/** "≈ 0.3 ¢", "cost unknown", or nothing while the estimate loads. */
function useCostText(estimate: CostEstimate | undefined): string | null {
  const { t, i18n } = useTranslation();
  if (!estimate) return null;
  if (estimate.cost_usd === null) return t("tafsir_ai.cost_unknown");
  return `≈ ${formatCost(estimate.cost_usd, i18n.resolvedLanguage)}`;
}

/** Machine text in a labelled box: saved, or still streaming. */
function MachineText(
  props:
    | { translation: TafsirTranslation; onLabelClick?: () => void }
    | { streamingText: string; mode: TranslationMode },
) {
  const { t, i18n } = useTranslation();
  const saved = "translation" in props ? props.translation : null;
  const mode = saved ? saved.mode : "mode" in props ? props.mode : "summary";
  const kind =
    mode === "summary" ? t("tafsir_ai.machine_summary") : t("tafsir_ai.machine_translation");
  const text = saved ? saved.text : "streamingText" in props ? props.streamingText : "";
  const label = saved
    ? t("tafsir_ai.label", {
        kind,
        source: `${providerName(saved.provider)}/${saved.model}`,
        date: new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium" }).format(
          new Date(saved.created_at),
        ),
      })
    : kind;
  const onLabelClick = "onLabelClick" in props ? props.onLabelClick : undefined;
  return (
    <div
      dir="ltr"
      lang={saved?.target_lang ?? i18n.resolvedLanguage}
      className="grid gap-1 rounded-md border border-dashed bg-muted/40 p-2 text-sm"
    >
      {onLabelClick ? (
        <button
          type="button"
          className="justify-self-start text-start text-xs text-muted-foreground underline-offset-2 hover:underline"
          onClick={onLabelClick}
        >
          {label}
        </button>
      ) : (
        <span className="text-xs text-muted-foreground">{label}</span>
      )}
      <p className="whitespace-pre-wrap" aria-busy={!saved}>
        {text || "…"}
      </p>
    </div>
  );
}

function providerName(provider: string): string {
  return provider in PROVIDER_NAMES ? PROVIDER_NAMES[provider as ProviderId] : provider;
}
