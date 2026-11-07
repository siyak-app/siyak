import { Link } from "@tanstack/react-router";
import { Maximize2Icon, PlusIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useUiStore } from "@/stores/ui";

import { ChatConversation } from "./ChatConversation";

/**
 * The Chat tab of the context panel: the current session beside the reader,
 * with a way to start a new one or open it on the Chat screen.
 */
export function ChatPanel() {
  const { t } = useTranslation();
  const sessionId = useUiStore((state) => state.chatSession);
  const setChatSession = useUiStore((state) => state.setChatSession);
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 pt-2">
      <div className="flex gap-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setChatSession(null);
          }}
        >
          <PlusIcon aria-hidden />
          {t("chat.new")}
        </Button>
        <Button asChild variant="ghost" size="sm">
          {sessionId === null ? (
            <Link to="/chat">
              <Maximize2Icon aria-hidden />
              {t("chat.open_full")}
            </Link>
          ) : (
            <Link to="/chat/$id" params={{ id: sessionId }}>
              <Maximize2Icon aria-hidden />
              {t("chat.open_full")}
            </Link>
          )}
        </Button>
      </div>
      <ChatConversation
        key={sessionId ?? "new"}
        sessionId={sessionId}
        onSession={setChatSession}
        compact
      />
    </div>
  );
}
