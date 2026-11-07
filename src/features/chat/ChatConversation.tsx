import { Link } from "@tanstack/react-router";
import { FolderPlusIcon, SendIcon, SquareIcon } from "lucide-react";
import { type KeyboardEvent, type SyntheticEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { formatCost } from "@/features/ai/formatCost";
import { useStream } from "@/features/ai/useStream";
import { PROVIDER_NAMES } from "@/features/related/providerNames";
import { SemanticRequired } from "@/features/related/SemanticRequired";
import { verseRange } from "@/features/tafsir/tafsirRefs";
import type {
  AiStatus,
  ChatMessage,
  ChatSessionSummary,
  ChatSources,
  ProviderId,
} from "@/lib/bindings";
import {
  sendChat,
  useAiStatus,
  useChatAnswered,
  useChatEstimate,
  useChatSession,
  useSemanticStatus,
  useUpsertNote,
} from "@/lib/queries";
import { useUiStore } from "@/stores/ui";

import { AnswerText } from "./AnswerText";

/** How long typing must pause before the cost of a message is estimated. */
const ESTIMATE_DELAY_MS = 400;

/**
 * One chat session (spec §10, §13.3): its messages with clickable citations
 * and "sources used", the composer with the estimated cost, and the
 * persistent notice that answers are machine output. `sessionId` is `null`
 * for a new chat; `onSession` learns the id once the first answer is saved.
 */
export function ChatConversation({
  sessionId,
  onSession,
  compact = false,
}: {
  sessionId: number | null;
  onSession: (id: number) => void;
  compact?: boolean;
}) {
  const { t, i18n } = useTranslation();
  const ai = useAiStatus();
  const semantic = useSemanticStatus();
  const session = useChatSession(sessionId);
  const answered = useChatAnswered();
  const { streaming, start, stop } = useStream<string>();
  const [error, setError] = useState<unknown>(null);
  const [draft, setDraft] = useState("");
  const end = useRef<HTMLDivElement>(null);

  const messages = session.data?.messages ?? [];
  const summary = session.data?.session ?? null;
  const ready = ai.data?.state === "ready" && semantic.data?.state === "ready";

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [messages.length, streaming?.text]);

  const send = async (message: string) => {
    setError(null);
    setDraft("");
    try {
      const reply = await start(message, (requestId, channel) =>
        sendChat(
          { sessionId, message, language: i18n.resolvedLanguage === "tr" ? "tr" : "en" },
          requestId,
          channel,
        ),
      );
      await answered(reply.session_id);
      if (reply.session_id !== null && reply.session_id !== sessionId) onSession(reply.session_id);
      if (reply.answer === null) setDraft(message);
    } catch (caught) {
      setDraft(message);
      setError(caught);
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div
        className="grid min-h-0 flex-1 content-start gap-4 overflow-y-auto pe-1"
        aria-live="polite"
        aria-busy={streaming !== null}
      >
        {session.isError && <ErrorMessage error={session.error} />}
        {messages.length === 0 && !streaming && (
          <p className="text-sm text-muted-foreground">{t("chat.empty")}</p>
        )}
        {messages.map((message) =>
          message.role === "user" ? (
            <Question key={message.id} text={message.content} />
          ) : (
            <Answer key={message.id} message={message} session={summary} compact={compact} />
          ),
        )}
        {streaming && (
          <>
            <Question text={streaming.key} />
            <div className="grid gap-1" role="status">
              {streaming.text ? (
                <AnswerText>{streaming.text}</AnswerText>
              ) : (
                <p className="text-sm text-muted-foreground">{t("chat.thinking")}</p>
              )}
            </div>
          </>
        )}
        <div ref={end} />
      </div>
      {error !== null && <ErrorMessage error={error} />}
      {ai.data && ai.data.state !== "ready" && <AiRequired status={ai.data} />}
      {ai.data?.state === "ready" && semantic.data && semantic.data.state !== "ready" && (
        <SemanticRequired status={semantic.data} />
      )}
      <Composer
        sessionId={sessionId}
        draft={draft}
        onDraft={setDraft}
        disabled={!ready}
        busy={streaming !== null}
        onSend={(message) => {
          void send(message);
        }}
        onStop={stop}
      />
      <p className="text-xs text-muted-foreground">{t("chat.disclaimer")}</p>
    </div>
  );
}

function Question({ text }: { text: string }) {
  return (
    <p
      dir="auto"
      className="max-w-[85%] justify-self-end rounded-lg bg-muted px-3 py-2 text-sm whitespace-pre-wrap"
    >
      {text}
    </p>
  );
}

function Answer({
  message,
  session,
  compact,
}: {
  message: ChatMessage;
  session: ChatSessionSummary | null;
  compact: boolean;
}) {
  const { t } = useTranslation();
  const skipped = message.sources?.notices.some(
    (notice) => notice.kind === "arabic_tafsirs_skipped",
  );
  return (
    <article className="grid gap-2">
      <AnswerText>{message.content}</AnswerText>
      {skipped && <p className="text-xs text-muted-foreground">{t("chat.arabic_skipped")}</p>}
      {message.cited.length > 0 && (
        <p className="flex flex-wrap items-baseline gap-1 text-xs">
          <span className="text-muted-foreground">{t("chat.cited")}</span>
          {message.cited.map((cited) => (
            <Link
              key={cited.verse_id}
              to="/read/$surah/$ayah"
              params={cited.verse}
              className="rounded border px-1 tabular-nums hover:bg-muted"
            >
              {cited.verse.surah}:{cited.verse.ayah}
            </Link>
          ))}
        </p>
      )}
      <div className="flex flex-wrap items-center gap-2">
        {message.sources && <Sources sources={message.sources} compact={compact} />}
        <SaveAnswer message={message} session={session} />
      </div>
    </article>
  );
}

/**
 * "Sources used": the search query the verses were found with, and every verse
 * and tafsir passage that went into the context.
 */
function Sources({ sources, compact }: { sources: ChatSources; compact: boolean }) {
  const { t } = useTranslation();
  return (
    <details className="w-full text-xs">
      <summary className="cursor-pointer text-muted-foreground">
        {t("chat.sources", { count: sources.verses.length + sources.tafsirs.length })}
      </summary>
      <div className={compact ? "grid gap-2 pt-2" : "grid gap-2 pt-2 sm:grid-cols-2"}>
        {sources.search_query && (
          <p className="text-muted-foreground sm:col-span-2">
            {t("chat.sources_query", { query: sources.search_query })}
          </p>
        )}
        <div className="grid content-start gap-1">
          <p className="font-medium">{t("chat.sources_verses")}</p>
          <ul className="flex flex-wrap gap-1">
            {sources.verses.map((verse) => (
              <li key={verse.verse_id}>
                <Link
                  to="/read/$surah/$ayah"
                  params={verse.verse}
                  className="rounded border px-1 tabular-nums hover:bg-muted"
                >
                  {verse.verse.surah}:{verse.verse.ayah}
                </Link>
              </li>
            ))}
          </ul>
        </div>
        {sources.tafsirs.length > 0 && (
          <div className="grid content-start gap-1">
            <p className="font-medium">{t("chat.sources_tafsirs")}</p>
            <ul className="grid gap-0.5">
              {sources.tafsirs.map((tafsir) => (
                <li key={`${tafsir.pack_id}/${String(tafsir.tafsir_id)}`}>
                  {tafsir.title} · {verseRange(tafsir.verse_from, tafsir.verse_to)}
                  {!tafsir.complete && (
                    <span className="text-muted-foreground"> {t("chat.sources_partial")}</span>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}
        {sources.notices.map((notice) =>
          notice.kind === "tafsir_missing" ? (
            <p key={notice.pack_id} className="text-muted-foreground">
              {t("chat.tafsir_missing", { pack: notice.pack_id })}
            </p>
          ) : null,
        )}
      </div>
    </details>
  );
}

/**
 * Saves an answer as a note on the first verse it cites, labelled as an AI
 * answer with its model and date (spec §10.7), then offers to add it to a
 * collection. An answer citing no verse has nothing to attach the note to.
 */
function SaveAnswer({
  message,
  session,
}: {
  message: ChatMessage;
  session: ChatSessionSummary | null;
}) {
  const { t, i18n } = useTranslation();
  const upsert = useUpsertNote();
  const openCollectionPicker = useUiStore((state) => state.openCollectionPicker);
  const first = message.cited[0];
  if (!first || !session) return null;
  const save = () => {
    const date = new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium" }).format(
      new Date(message.created_at),
    );
    const provider =
      session.provider in PROVIDER_NAMES
        ? PROVIDER_NAMES[session.provider as ProviderId]
        : session.provider;
    const body = `**${t("chat.ai_answer")}** · ${provider}/${session.model} · ${date}\n\n${message.content}`;
    upsert.mutate(
      { id: null, verse_from: first.verse_id, verse_to: first.verse_id, body },
      {
        onSuccess: (note) => {
          openCollectionPicker({ kind: "note", id: note.id });
        },
      },
    );
  };
  return (
    <Button variant="ghost" size="sm" disabled={upsert.isPending} onClick={save}>
      <FolderPlusIcon aria-hidden />
      {t("chat.add_to_collection")}
    </Button>
  );
}

function AiRequired({ status }: { status: Exclude<AiStatus, { state: "ready" }> }) {
  const { t } = useTranslation();
  const setSettingsTab = useUiStore((state) => state.setSettingsTab);
  const message =
    status.state === "key_missing"
      ? t("ai.key_missing", { provider: PROVIDER_NAMES[status.provider] })
      : status.state === "base_url_missing"
        ? t("ai.base_url_missing")
        : status.state === "base_url_invalid"
          ? t("semantic.base_url_invalid")
          : t("chat.ai_required");
  const tab = status.state === "no_provider" || status.state === "no_model" ? "ai" : "providers";
  return (
    <div className="grid justify-items-start gap-2 rounded-lg border p-3">
      <p className="text-sm">{message}</p>
      <Button
        variant="outline"
        size="sm"
        onClick={() => {
          setSettingsTab(tab);
        }}
      >
        {t("chat.open_settings")}
      </Button>
    </div>
  );
}

function Composer({
  sessionId,
  draft,
  onDraft,
  disabled,
  busy,
  onSend,
  onStop,
}: {
  sessionId: number | null;
  draft: string;
  onDraft: (draft: string) => void;
  disabled: boolean;
  busy: boolean;
  onSend: (message: string) => void;
  onStop: () => void;
}) {
  const { t, i18n } = useTranslation();
  const [settled, setSettled] = useState("");
  const [confirming, setConfirming] = useState(false);
  const message = draft.trim();

  useEffect(() => {
    const timer = setTimeout(() => {
      setSettled(message);
    }, ESTIMATE_DELAY_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [message]);

  const estimate = useChatEstimate(sessionId, settled, !disabled && settled !== "");
  const cost = estimate.data?.cost_usd;

  const submit = (event?: SyntheticEvent) => {
    event?.preventDefault();
    if (!message || disabled || busy) return;
    if (estimate.data?.over_limit && !confirming) {
      setConfirming(true);
      return;
    }
    setConfirming(false);
    onSend(message);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <form className="grid gap-1.5" onSubmit={submit}>
      <label htmlFor="chat-message" className="sr-only">
        {t("chat.message_label")}
      </label>
      <div className="flex items-end gap-2">
        <Textarea
          id="chat-message"
          dir="auto"
          rows={2}
          className="max-h-40"
          placeholder={t("chat.placeholder")}
          disabled={disabled}
          value={draft}
          onChange={(event) => {
            onDraft(event.target.value);
            setConfirming(false);
          }}
          onKeyDown={onKeyDown}
        />
        {busy ? (
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={onStop}
            aria-label={t("chat.stop")}
          >
            <SquareIcon />
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            disabled={disabled || !message}
            aria-label={t("chat.send")}
          >
            <SendIcon />
          </Button>
        )}
      </div>
      {confirming ? (
        <p className="flex flex-wrap items-center gap-2 text-xs" role="alert">
          {t("chat.over_limit")}
          <Button type="submit" size="sm">
            {t("tafsir_ai.continue")}
          </Button>
        </p>
      ) : (
        message !== "" &&
        estimate.data && (
          <p className="text-xs text-muted-foreground">
            {cost === null || cost === undefined
              ? t("chat.cost_unknown")
              : t("chat.cost", { cost: formatCost(cost, i18n.resolvedLanguage) })}
          </p>
        )
      )}
    </form>
  );
}
