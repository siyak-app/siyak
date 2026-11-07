import { Link, useNavigate } from "@tanstack/react-router";
import { PencilIcon, PlusIcon, Trash2Icon } from "lucide-react";
import { type SyntheticEvent, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorMessage } from "@/components/ErrorMessage";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ChatSessionSummary } from "@/lib/bindings";
import { useChatSessions, useDeleteChatSession, useRenameChatSession } from "@/lib/queries";

import { ChatConversation } from "./ChatConversation";

/** The Chat screen (spec §13.3): the sessions beside the open one. */
export function ChatView({ sessionId }: { sessionId: number | null }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <div className="flex h-full min-h-0 gap-4 p-4">
      <SessionList active={sessionId} />
      <section
        aria-label={t("chat.title")}
        className="mx-auto flex min-h-0 w-full max-w-3xl flex-1 flex-col"
      >
        <ChatConversation
          key={sessionId ?? "new"}
          sessionId={sessionId}
          onSession={(id) => {
            void navigate({ to: "/chat/$id", params: { id } });
          }}
        />
      </section>
    </div>
  );
}

function SessionList({ active }: { active: number | null }) {
  const { t } = useTranslation();
  const sessions = useChatSessions();
  return (
    <nav aria-label={t("chat.sessions")} className="hidden w-56 shrink-0 flex-col gap-2 md:flex">
      <Button asChild variant="outline" size="sm" className="justify-start">
        <Link to="/chat">
          <PlusIcon aria-hidden />
          {t("chat.new")}
        </Link>
      </Button>
      {sessions.isError && <ErrorMessage error={sessions.error} />}
      <ul className="grid min-h-0 content-start gap-0.5 overflow-y-auto">
        {sessions.data?.map((session) => (
          <SessionRow key={session.id} session={session} active={session.id === active} />
        ))}
      </ul>
    </nav>
  );
}

function SessionRow({ session, active }: { session: ChatSessionSummary; active: boolean }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const remove = useDeleteChatSession();
  const rename = useRenameChatSession();
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(session.title ?? "");
  const name = session.title?.trim() ? session.title : t("chat.untitled");

  const submit = (event: SyntheticEvent) => {
    event.preventDefault();
    if (!title.trim()) return;
    rename.mutate(
      { id: session.id, title },
      {
        onSuccess: () => {
          setEditing(false);
        },
      },
    );
  };

  if (editing) {
    return (
      <li>
        <form onSubmit={submit} className="flex gap-1">
          <Input
            aria-label={t("chat.rename_label")}
            value={title}
            autoFocus
            onChange={(event) => {
              setTitle(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Escape") setEditing(false);
            }}
          />
        </form>
      </li>
    );
  }
  return (
    <li className="group flex items-center gap-0.5">
      <Link
        to="/chat/$id"
        params={{ id: session.id }}
        aria-current={active ? "page" : undefined}
        className="min-w-0 flex-1 truncate rounded-md px-2 py-1.5 text-sm hover:bg-muted aria-[current=page]:bg-primary/10"
      >
        {name}
      </Link>
      <Button
        variant="ghost"
        size="icon-sm"
        className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        aria-label={t("chat.rename", { title: name })}
        onClick={() => {
          setTitle(session.title ?? "");
          setEditing(true);
        }}
      >
        <PencilIcon />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        aria-label={t("chat.delete", { title: name })}
        disabled={remove.isPending}
        onClick={() => {
          remove.mutate(session.id, {
            onSuccess: () => {
              if (active) void navigate({ to: "/chat" });
            },
          });
        }}
      >
        <Trash2Icon />
      </Button>
    </li>
  );
}
