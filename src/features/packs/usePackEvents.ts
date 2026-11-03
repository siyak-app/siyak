import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { create } from "zustand";

import { announce } from "@/lib/announce";
import {
  type AppErrorCode,
  type Catalog,
  PACK_DONE_EVENT,
  PACK_ERROR_EVENT,
  PACK_PROGRESS_EVENT,
  type PackDone,
  type PackFailed,
  type PackProgress,
} from "@/lib/bindings";
import { invalidateContent, queryKeys } from "@/lib/queries";

import { packTitle, withJob } from "./packFormat";

interface PackFailures {
  /** The last failure of each pack, until it is queued again. */
  failures: Partial<Record<string, AppErrorCode>>;
}

/** Why the last job of a pack failed (cancellations are not kept). */
export const usePackFailures = create<PackFailures>()(() => ({ failures: {} }));

function setFailure(id: string, code: AppErrorCode | undefined) {
  usePackFailures.setState((state) => ({ failures: { ...state.failures, [id]: code } }));
}

/**
 * Follows the core's pack jobs (spec §12 events): progress is patched into
 * the cached catalog, and a finished job refreshes everything that depends on
 * the installed packs. Mounted once, in the app shell.
 */
export function usePackEvents() {
  const queryClient = useQueryClient();
  const { t, i18n } = useTranslation();

  useEffect(() => {
    const title = (id: string) => {
      const pack = queryClient
        .getQueryData<Catalog>(queryKeys.catalog)
        ?.packs.find((p) => p.id === id);
      return pack ? packTitle(pack.title, pack.title_fallback, i18n.resolvedLanguage) : id;
    };
    const subscriptions = [
      listen<PackProgress>(PACK_PROGRESS_EVENT, ({ payload }) => {
        setFailure(payload.id, undefined);
        queryClient.setQueryData<Catalog>(
          queryKeys.catalog,
          (catalog) => catalog && withJob(catalog, payload.id, payload.job),
        );
      }),
      listen<PackDone>(PACK_DONE_EVENT, ({ payload }) => {
        announce(t("packs.announce_installed", { title: title(payload.id) }));
        void invalidateContent(queryClient);
      }),
      listen<PackFailed>(PACK_ERROR_EVENT, ({ payload }) => {
        const cancelled = payload.error.code === "install_cancelled";
        if (!cancelled) {
          setFailure(payload.id, payload.error.code);
          announce(t("packs.announce_failed", { title: title(payload.id) }));
        }
        queryClient.setQueryData<Catalog>(
          queryKeys.catalog,
          (catalog) => catalog && withJob(catalog, payload.id, null),
        );
        void queryClient.invalidateQueries({ queryKey: queryKeys.catalog });
      }),
    ];
    return () => {
      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => {
          unlisten();
        });
      }
    };
  }, [queryClient, t, i18n]);
}
