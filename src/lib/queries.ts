/**
 * TanStack Query hooks around the generated IPC bindings.
 *
 * Components use these hooks and never call `commands` from `./bindings`
 * directly, so caching, query keys and error handling live in one place.
 *
 * @packageDocumentation
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type AppError,
  type AppErrorCode,
  type CoreStatus,
  type Surah,
  type VerseWithTranslations,
} from "./bindings";

/** The result shape every generated command returns. */
type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

/**
 * An `AppError` returned by the Rust core, thrown so TanStack Query
 * treats it as a failed query.
 */
export class AppErrorException extends Error {
  /** Stable error code; map it to a message with `errors.<code>`. */
  readonly code: AppErrorCode;

  /** Wraps an error returned by a command. */
  constructor(error: AppError) {
    super(`${error.code}: ${error.message}`);
    this.name = "AppErrorException";
    this.code = error.code;
  }
}

/**
 * Returns the stable code of an error thrown by a query or mutation, or
 * `undefined` for errors that did not come from the Rust core.
 */
export function appErrorCode(error: unknown): AppErrorCode | undefined {
  return error instanceof AppErrorException ? error.code : undefined;
}

async function unwrap<T>(result: Promise<CommandResult<T>>): Promise<T> {
  const settled = await result;
  if (settled.status === "error") throw new AppErrorException(settled.error);
  return settled.data;
}

/** Query keys, exported so callers can invalidate or prefill caches. */
export const queryKeys = {
  /** Key of {@link useSurahs}. */
  surahs: ["surahs"] as const,
  /** Key of {@link useVerses} for one surah and translation selection. */
  verses: (surah: number, translationIds: readonly string[]) =>
    ["verses", surah, [...translationIds]] as const,
  /** Key of {@link useSettings}. */
  settings: ["settings"] as const,
  /** Key of {@link useCoreStatus}. */
  coreStatus: ["core-status"] as const,
};

/** All 114 surahs. Content changes only when packs change, so it never goes stale. */
export function useSurahs() {
  return useQuery<Surah[]>({
    queryKey: queryKeys.surahs,
    queryFn: () => unwrap(commands.listSurahs()),
    staleTime: Infinity,
  });
}

/**
 * The verses of `surah` with the given translations, in that order.
 * Unknown translation ids are ignored by the core.
 */
export function useVerses(surah: number, translationIds: readonly string[]) {
  return useQuery<VerseWithTranslations[]>({
    queryKey: queryKeys.verses(surah, translationIds),
    queryFn: () => unwrap(commands.getVerses(surah, [...translationIds])),
    staleTime: Infinity,
  });
}

/**
 * The core data version, the debug dataset (`dev`/`full`) and any error from
 * refreshing the core data at startup. Fixed for the lifetime of the app.
 */
export function useCoreStatus() {
  return useQuery<CoreStatus>({
    queryKey: queryKeys.coreStatus,
    queryFn: () => unwrap(commands.getCoreStatus()),
    staleTime: Infinity,
  });
}

/** Every stored setting as `key → JSON text`. */
export function useSettings() {
  return useQuery<Partial<Record<string, string>>>({
    queryKey: queryKeys.settings,
    queryFn: () => unwrap(commands.getSettings()),
    staleTime: Infinity,
  });
}

/** Variables of {@link useSetSetting}. */
export interface SetSettingVariables {
  /** Dotted setting key, e.g. `ui.language`. */
  key: string;
  /** Any JSON-serializable value. */
  value: unknown;
}

/** Stores one setting and updates the cached settings. */
export function useSetSetting() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, SetSettingVariables>({
    mutationFn: ({ key, value }) => unwrap(commands.setSetting(key, JSON.stringify(value))),
    onSuccess: (_data, { key, value }) => {
      queryClient.setQueryData<Partial<Record<string, string>>>(queryKeys.settings, (old) => ({
        ...old,
        [key]: JSON.stringify(value),
      }));
    },
  });
}
