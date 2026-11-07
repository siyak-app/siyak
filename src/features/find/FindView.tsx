import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { SearchIcon, SquareIcon } from "lucide-react";
import { type KeyboardEvent, type SyntheticEvent, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { EmbeddingPicker } from "@/features/ai/EmbeddingPicker";
import { LlmPicker } from "@/features/ai/LlmPicker";
import { AiRequired } from "@/features/chat/ChatConversation";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import { RelatedVerseList } from "@/features/related/RelatedVerseList";
import { SemanticRequired } from "@/features/related/SemanticRequired";
import { CheckField } from "@/features/settings/fields";
import { queryKeys, useAiStatus, useFindVerses, useSemanticStatus } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

/** Verses asked for at once; "Show more" reveals them a page at a time. */
const FIND_MAX = 50;
const FIND_PAGE = 20;

/**
 * The Find screen: the verses closest in meaning to a question, with no answer
 * written (spec §7.4). The chat model may first turn the question into a search
 * sentence, as chat does (spec §10.1); the answer and search models are chosen
 * at the top. `q` is the question last searched for.
 */
export function FindView({ q }: { q: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [draft, setDraft] = useState(q);
  const rewrite = useUiStore((state) => state.preferences["find.rewrite"]);
  const setPreference = useUiStore((state) => state.setPreference);
  const ai = useAiStatus();
  const semantic = useSemanticStatus();
  const aiReady = ai.data?.state === "ready";
  const semanticReady = semantic.data?.state === "ready";

  const submit = (event?: SyntheticEvent) => {
    event?.preventDefault();
    const query = draft.trim();
    if (!query || !semanticReady) return;
    void navigate({ to: "/find", search: { q: query } });
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto grid max-w-3xl gap-4 p-6">
        <header className="grid gap-1">
          <h1 className="text-lg font-semibold">{t("find.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("find.hint")}</p>
        </header>
        <section
          aria-label={t("models.bar_label")}
          className="grid gap-3 rounded-lg border p-3 text-sm"
        >
          <EmbeddingPicker compact />
          <CheckField
            label={t("find.rewrite")}
            hint={t("find.rewrite_hint")}
            checked={rewrite}
            onChange={(checked) => {
              setPreference("find.rewrite", checked);
            }}
          />
          {rewrite && <LlmPicker />}
          {rewrite && ai.data && ai.data.state !== "ready" && <AiRequired status={ai.data} />}
        </section>
        {semantic.data && semantic.data.state !== "ready" && (
          <SemanticRequired status={semantic.data} />
        )}
        <form className="grid gap-2" onSubmit={submit}>
          <label htmlFor="find-query" className="sr-only">
            {t("find.query_label")}
          </label>
          <div className="flex items-end gap-2">
            <Textarea
              id="find-query"
              dir="auto"
              rows={2}
              className="max-h-40"
              placeholder={t("find.placeholder")}
              value={draft}
              onChange={(event) => {
                setDraft(event.target.value);
              }}
              onKeyDown={onKeyDown}
            />
            <Button type="submit" disabled={!draft.trim() || !semanticReady}>
              <SearchIcon aria-hidden />
              {t("find.submit")}
            </Button>
          </div>
        </form>
        {q !== "" && semanticReady && <FindResults key={q} q={q} rewrite={rewrite && aiReady} />}
      </div>
    </div>
  );
}

function FindResults({ q, rewrite }: { q: string; rewrite: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [shown, setShown] = useState(FIND_PAGE);
  const [stopped, setStopped] = useState(false);
  const semantic = useSemanticStatus();
  const translationId = useUiStore((state) => state.preferences["reader.translations"][0] ?? null);
  const pack = useUiStore((state) => state.preferences["semantic.vector_pack"]);
  const provider = useUiStore((state) => state.preferences["ai.provider"]);
  const chatModel = useUiStore((state) => state.preferences["ai.chat_model"]);
  const variables = {
    query: q,
    rewrite,
    translationId,
    limit: FIND_MAX,
    models: [pack ?? "", rewrite ? `${provider ?? ""}/${chatModel ?? ""}` : ""].join("|"),
  };
  const results = useFindVerses(variables);
  const remote = semantic.data?.state === "ready" ? semantic.data.provider : null;

  if (results.isError) return <ErrorMessage error={results.error} />;
  if ((stopped && !results.isFetching && !results.data) || results.data?.cancelled) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        {t("find.stopped")}
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setStopped(false);
            void results.refetch();
          }}
        >
          {t("find.again")}
        </Button>
      </div>
    );
  }
  if (results.isFetching && !results.data) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground" role="status">
        {rewrite ? t("find.rewriting") : t("search.meaning_searching")}
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setStopped(true);
            void queryClient.cancelQueries({ queryKey: queryKeys.findVerses(variables) });
          }}
        >
          <SquareIcon aria-hidden />
          {t("chat.stop")}
        </Button>
      </div>
    );
  }
  if (!results.data) return null;
  const { search_query: searchQuery, verses } = results.data;
  return (
    <section className="grid gap-3" aria-label={t("find.results")}>
      <p className="text-sm text-muted-foreground" role="status">
        {searchQuery
          ? t("find.searched_for", { query: searchQuery })
          : t("find.count", { count: verses.length })}
        {remote && ` ${t("semantic.remote", { provider: PROVIDER_NAMES[remote] })}`}
      </p>
      {verses.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("search.no_results")}</p>
      ) : (
        <RelatedVerseList verses={verses.slice(0, shown)} />
      )}
      {verses.length > shown && (
        <Button
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() => {
            setShown(shown + FIND_PAGE);
          }}
        >
          {t("search.show_more")}
        </Button>
      )}
    </section>
  );
}
