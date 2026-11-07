/**
 * TanStack Query hooks around the generated IPC bindings.
 *
 * Components use these hooks and never call `commands` from `./bindings`
 * directly, so caching, query keys and error handling live in one place.
 *
 * @packageDocumentation
 */
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Channel } from "@tauri-apps/api/core";

import { serializeSetting, type SettingKey, type Settings } from "./settings";

import {
  commands,
  type AiStatus,
  type ApiKeyStatus,
  type AppError,
  type AppErrorCode,
  type BookmarkedVerse,
  type ChatReply,
  type ChatSession,
  type ChatSessionSummary,
  type Catalog,
  type CitationRef,
  type CitationStyle,
  type Collection,
  type CollectionInput,
  type CollectionSummary,
  type Compare,
  type Concordance,
  type ConcordanceTarget,
  type CostEstimate,
  type CoreStatus,
  type ImportMode,
  type ImportReport,
  type ItemRef,
  type MarkdownLabels,
  type ModelInfo,
  type MorphologyPack,
  type NetworkLogEntry,
  type Note,
  type NoteInput,
  type ProviderId,
  type QueryClass,
  type RelatedVerse,
  type RootSearch,
  type SearchFilters,
  type SearchResults,
  type SearchScope,
  type SemanticStatus,
  type Surah,
  type TafsirInfo,
  type TafsirTranslation,
  type TafsirPassage,
  type TranslationInfo,
  type TranslationMode,
  type TranslateResult,
  type UsagePeriod,
  type UsageSummary,
  type VacuumSizes,
  type VerseIdRange,
  type VerseWithTranslations,
  type VerseWord,
  type WordDetail,
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

/** Variables of {@link useCompare}. */
export interface CompareVariables {
  /** Global id of the first verse. */
  from: number;
  /** Global id of the last verse. */
  to: number;
  /** Translation pack ids. */
  translationIds: string[];
  /** Tafsir pack ids. */
  tafsirIds: string[];
}

/** Variables of {@link useSearchText}. */
export interface SearchTextVariables {
  /** The query as typed. */
  query: string;
  /** The Arabic text or the translations. */
  scope: SearchScope;
  /** Translation packs to search; empty for all of them. */
  packIds: string[];
  /** Surah limits. */
  filters: SearchFilters;
  /** Most results to return. */
  limit: number;
}

/** Variables of {@link useSearchMeaning}. */
export interface MeaningVariables {
  /** The query as typed. */
  query: string;
  /** The translation to show with each verse. */
  translationId: string | null;
  /** Most verses to return. */
  limit: number;
}

/** Variables of {@link useConcordance}. */
export interface ConcordanceVariables {
  /** The root or lemma. */
  target: ConcordanceTarget;
  /** The translation to show with each verse. */
  translationId: string | null;
  /** Most verses to return. */
  limit: number;
}

/** Variables of {@link useAddCollectionItem}. */
export interface AddCollectionItemVariables {
  /** The collection to add to. */
  collectionId: number;
  /** What to add. */
  item: ItemRef;
  /** Short note on the item. */
  annotation: string | null;
}

/** Variables of {@link useMoveCollectionItem}. */
export interface MoveCollectionItemVariables {
  /** The item's collection, whose cached order changes at once. */
  collectionId: number;
  /** The item. */
  id: number;
  /** Its new position, from 0. */
  toIndex: number;
}

/** Variables of {@link useExportCollectionMarkdown}. */
export interface ExportMarkdownVariables {
  /** The collection. */
  id: number;
  /** The translations to include. */
  translationIds: string[];
  /** The words of the file, in the interface language. */
  labels: MarkdownLabels;
  /** Citation style of the bibliography. */
  style: string;
}

/** Query keys, exported so callers can invalidate or prefill caches. */
export const queryKeys = {
  /** Key of {@link useSurahs}. */
  surahs: ["surahs"] as const,
  /** Key of {@link useSurah}. */
  surah: (number: number) => ["surah", number] as const,
  /** Key of {@link useTranslations}. */
  translations: ["translations"] as const,
  /** Key of {@link useVerses} for one surah and translation selection. */
  verses: (surah: number, translationIds: readonly string[]) =>
    ["verses", surah, [...translationIds]] as const,
  /** Key of {@link useSettings}. */
  settings: ["settings"] as const,
  /** Key of {@link useCoreStatus}. */
  coreStatus: ["core-status"] as const,
  /** Key of {@link useTafsirs}. */
  tafsirs: ["tafsirs"] as const,
  /** Key of {@link useTafsir}. */
  tafsir: (packId: string, verseId: number) => ["tafsir", packId, verseId] as const,
  /** Key of {@link useTafsirIntroduction}. */
  tafsirIntroduction: (packId: string) => ["tafsir-introduction", packId] as const,
  /** Key of {@link useCompare}. */
  compare: (variables: CompareVariables) => ["compare", variables] as const,
  /** Key of {@link useClassifyQuery}. */
  classify: (query: string) => ["classify", query] as const,
  /** Key of {@link useSearchText}. */
  search: (variables: SearchTextVariables) => ["search", variables] as const,
  /** Key of {@link useMorphologyPack}. */
  morphologyPack: ["morphology-pack"] as const,
  /** Key of {@link useVerseWords}. */
  verseWords: (verseId: number | null) => ["verse-words", verseId] as const,
  /** Key of {@link useWord}. */
  word: (wordId: number) => ["word", wordId] as const,
  /** Key of {@link useConcordance}. */
  concordance: (variables: ConcordanceVariables) => ["concordance", variables] as const,
  /** Key of {@link useSearchRoots}. */
  searchRoots: (query: string) => ["search-roots", query] as const,
  /** Key of {@link useSemanticStatus}. */
  semanticStatus: ["semantic-status"] as const,
  /** Key of {@link useSearchMeaning}. */
  searchMeaning: (variables: MeaningVariables) => ["search-meaning", variables] as const,
  /** Key of {@link useRelatedByMeaning}. */
  relatedByMeaning: (verseId: number | null, translationId: string | null) =>
    ["related-by-meaning", verseId, translationId] as const,
  /** Key of {@link useRelatedByRoots}. */
  relatedByRoots: (verseId: number | null, translationId: string | null) =>
    ["related-by-roots", verseId, translationId] as const,
  /** Key of {@link useApiKeys}. */
  apiKeys: ["api-keys"] as const,
  /** Key of {@link useAiStatus}. */
  aiStatus: ["ai-status"] as const,
  /** Key of {@link useTafsirTranslations}. */
  tafsirTranslations: (packId: string, tafsirId: number, lang: string) =>
    ["tafsir-translations", packId, tafsirId, lang] as const,
  /** Key of {@link useTafsirEstimate}. */
  tafsirEstimate: (variables: TafsirJobVariables) => ["tafsir-estimate", variables] as const,
  /** Key of {@link useChatSessions}. */
  chatSessions: ["chat-sessions"] as const,
  /** Key of {@link useChatSession}. */
  chatSession: (id: number | null) => ["chat-session", id] as const,
  /** Key of {@link useChatEstimate}. */
  chatEstimate: (sessionId: number | null, message: string) =>
    ["chat-estimate", sessionId, message] as const,
  /** Key of {@link useUsageSummary}. */
  usageSummary: (period: UsagePeriod) => ["usage-summary", period] as const,
  /** Key of {@link useModels}. */
  models: (provider: ProviderId | null) => ["models", provider] as const,
  /** Key of {@link useCatalog}. */
  catalog: ["catalog"] as const,
  /** Key of {@link useNetworkLog}. */
  networkLog: ["network-log"] as const,
};

/** All 114 surahs. Content changes only when packs change, so it never goes stale. */
export function useSurahs() {
  return useQuery<Surah[]>({
    queryKey: queryKeys.surahs,
    queryFn: () => unwrap(commands.listSurahs()),
    staleTime: Infinity,
  });
}

/** One surah, including its basmala header (`null` for surahs 1 and 9). */
export function useSurah(number: number) {
  return useQuery<Surah>({
    queryKey: queryKeys.surah(number),
    queryFn: () => unwrap(commands.getSurah(number)),
    staleTime: Infinity,
  });
}

/** The installed translation packs, bundled ones first. */
export function useTranslations() {
  return useQuery<TranslationInfo[]>({
    queryKey: queryKeys.translations,
    queryFn: () => unwrap(commands.listTranslations()),
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
export interface SetSettingVariables<K extends SettingKey = SettingKey> {
  /** Dotted setting key, e.g. `ui.language`. */
  key: K;
  /** The typed value, serialized with `serializeSetting`. */
  value: Settings[K];
}

/** Stores one setting and updates the cached settings. */
export function useSetSetting() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, SetSettingVariables>({
    mutationFn: ({ key, value }) => unwrap(commands.setSetting(key, serializeSetting(key, value))),
    onSuccess: (_data, { key, value }) => {
      queryClient.setQueryData<Partial<Record<string, string>>>(queryKeys.settings, (old) => ({
        ...old,
        [key]: serializeSetting(key, value),
      }));
    },
  });
}

/** Serializes typed settings to the `key → JSON text` form `user.db` stores. */
export function serializeSettings(values: Partial<Settings>): Record<string, string> {
  return Object.fromEntries(
    (Object.keys(values) as SettingKey[]).map((key) => [
      key,
      serializeSetting(key, values[key] as Settings[typeof key]),
    ]),
  );
}

/**
 * Stores several settings in one transaction (all or none) and updates the
 * cached settings, e.g. every default of an experience profile.
 */
export function useSetSettings() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, Partial<Settings>>({
    mutationFn: (values) => unwrap(commands.setSettings(serializeSettings(values))),
    onSuccess: (_data, values) => {
      queryClient.setQueryData<Partial<Record<string, string>>>(queryKeys.settings, (old) => ({
        ...old,
        ...serializeSettings(values),
      }));
    },
  });
}

/** The installed tafsir packs. */
export function useTafsirs() {
  return useQuery<TafsirInfo[]>({
    queryKey: queryKeys.tafsirs,
    queryFn: () => unwrap(commands.listTafsirs()),
    staleTime: Infinity,
  });
}

/**
 * The commentary of a tafsir covering a verse, or `null` if it has none. The
 * previous passage stays up while moving between verses of the same tafsir.
 */
export function useTafsir(verseId: number, packId: string) {
  return useQuery<TafsirPassage | null>({
    queryKey: queryKeys.tafsir(packId, verseId),
    queryFn: () => unwrap(commands.getTafsir(verseId, packId)),
    staleTime: Infinity,
    placeholderData: (previous, query) =>
      query?.queryKey[1] === packId ? keepPreviousData(previous) : undefined,
  });
}

/** The author's introduction of a tafsir, or `null` if it has none. */
export function useTafsirIntroduction(packId: string) {
  return useQuery<TafsirPassage | null>({
    queryKey: queryKeys.tafsirIntroduction(packId),
    queryFn: () => unwrap(commands.getTafsirIntroduction(packId)),
    staleTime: Infinity,
  });
}

/** A range of verses with the chosen translations and tafsirs. */
export function useCompare(variables: CompareVariables) {
  const { from, to, translationIds, tafsirIds } = variables;
  return useQuery<Compare>({
    queryKey: queryKeys.compare(variables),
    queryFn: () => unwrap(commands.getCompare(from, to, translationIds, tafsirIds)),
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
}

const classifyOptions = (query: string) => ({
  queryKey: queryKeys.classify(query),
  queryFn: () => unwrap(commands.classifyQuery(query)),
  staleTime: Infinity,
});

/** What a query is: a verse to jump to, Arabic letters, a root or other text. */
export function useClassifyQuery(query: string) {
  return useQuery<QueryClass>(classifyOptions(query));
}

/** {@link useClassifyQuery} outside rendering, for the search box's submit. */
export function classifyQuery(
  queryClient: ReturnType<typeof useQueryClient>,
  query: string,
): Promise<QueryClass> {
  return queryClient.query(classifyOptions(query));
}

/**
 * Searches the Arabic text or the translations. The previous results stay up
 * while the filters or the length of the list change.
 */
export function useSearchText(variables: SearchTextVariables) {
  const { query, scope, packIds, filters, limit } = variables;
  return useQuery<SearchResults>({
    queryKey: queryKeys.search(variables),
    queryFn: () => unwrap(commands.searchText(query, scope, packIds, filters, limit)),
    staleTime: Infinity,
    placeholderData: (previous, previousQuery) => {
      const before = previousQuery?.queryKey[1] as SearchTextVariables | undefined;
      return before?.query === query && before.scope === scope ? previous : undefined;
    },
  });
}

/**
 * The Packs screen: manifest (fetched at most hourly by the core) plus what is
 * installed and queued. Enabled on demand, so the network is only used where
 * packs are shown. Job progress is patched in by `usePackEvents`.
 */
export function useCatalog(enabled = true) {
  return useQuery<Catalog>({
    queryKey: queryKeys.catalog,
    queryFn: () => unwrap(commands.fetchManifest(false)),
    staleTime: 60_000,
    enabled,
  });
}

/** The installed morphology pack; `null` without one. */
export function useMorphologyPack() {
  return useQuery<MorphologyPack | null>({
    queryKey: queryKeys.morphologyPack,
    queryFn: () => unwrap(commands.getMorphologyPack()),
    staleTime: Infinity,
  });
}

/** The words of a verse; waits while `verseId` is `null`. */
export function useVerseWords(verseId: number | null) {
  return useQuery<VerseWord[]>({
    queryKey: queryKeys.verseWords(verseId),
    queryFn: () => unwrap(commands.getVerseWords(verseId ?? 0)),
    staleTime: Infinity,
    enabled: verseId !== null,
  });
}

/** A word with its full analysis. */
export function useWord(wordId: number) {
  return useQuery<WordDetail>({
    queryKey: queryKeys.word(wordId),
    queryFn: () => unwrap(commands.getWord(wordId)),
    staleTime: Infinity,
  });
}

/**
 * Every occurrence of a root or lemma. The shown verses stay while more load;
 * the screen is remounted for another root, so it never shows the previous one.
 */
export function useConcordance(variables: ConcordanceVariables) {
  return useQuery<Concordance>({
    queryKey: queryKeys.concordance(variables),
    queryFn: () =>
      unwrap(commands.concordance(variables.target, variables.translationId, variables.limit, 0)),
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
}

/** Roots and lemmas matching a query. */
export function useSearchRoots(query: string, enabled = true) {
  return useQuery<RootSearch>({
    queryKey: queryKeys.searchRoots(query),
    queryFn: () => unwrap(commands.searchRoots(query)),
    staleTime: Infinity,
    enabled,
  });
}

/** Whether semantic search and cross-references work, and if not, why. */
export function useSemanticStatus() {
  return useQuery<SemanticStatus>({
    queryKey: queryKeys.semanticStatus,
    queryFn: () => unwrap(commands.getSemanticStatus()),
    staleTime: Infinity,
  });
}

/**
 * The verses closest in meaning to a query. The first search loads the model,
 * which takes a few seconds; a longer list keeps the shown verses meanwhile.
 */
export function useSearchMeaning(variables: MeaningVariables, enabled = true) {
  const { query, translationId, limit } = variables;
  return useQuery<RelatedVerse[]>({
    queryKey: queryKeys.searchMeaning(variables),
    queryFn: () => unwrap(commands.searchMeaning(query, translationId, limit)),
    staleTime: Infinity,
    enabled,
    placeholderData: (previous, previousQuery) => {
      const before = previousQuery?.queryKey[1] as MeaningVariables | undefined;
      return before?.query === query ? previous : undefined;
    },
  });
}

/** Everything that depends on the active embedder: its status and results. */
function invalidateSemantic(queryClient: ReturnType<typeof useQueryClient>) {
  return invalidateKeys(queryClient, [
    queryKeys.semanticStatus,
    ["search-meaning"],
    ["related-by-meaning"],
  ]);
}

/** Which providers have an API key in the keychain; never the keys themselves. */
export function useApiKeys() {
  return useQuery<ApiKeyStatus[]>({
    queryKey: queryKeys.apiKeys,
    queryFn: () => unwrap(commands.getApiKeys()),
    staleTime: Infinity,
  });
}

/** Variables of {@link useSetApiKey}. */
export interface SetApiKeyVariables {
  provider: ProviderId;
  key: string;
}

/** Saves a provider's API key in the keychain. */
export function useSetApiKey() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, SetApiKeyVariables>({
    mutationFn: ({ provider, key }) => unwrap(commands.setApiKey(provider, key)),
    onSuccess: () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys }),
        invalidateSemantic(queryClient),
        invalidateAi(queryClient),
      ]),
  });
}

/** Removes a provider's API key from the keychain. */
export function useDeleteApiKey() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, ProviderId>({
    mutationFn: (provider) => unwrap(commands.deleteApiKey(provider)),
    onSuccess: () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.apiKeys }),
        invalidateSemantic(queryClient),
        invalidateAi(queryClient),
      ]),
  });
}

/** A setting the semantic service reads: the active vector pack or the server address. */
export type SemanticSettingKey = "semantic.vector_pack" | "semantic.base_url";

/**
 * Stores a setting the semantic service reads and refreshes what depends on
 * it, so a new address or pack takes effect at once.
 */
export function useSetSemanticSetting() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, SetSettingVariables<SemanticSettingKey>>({
    mutationFn: ({ key, value }) => unwrap(commands.setSetting(key, serializeSetting(key, value))),
    onSuccess: async (_data, { key, value }) => {
      queryClient.setQueryData<Partial<Record<string, string>>>(queryKeys.settings, (old) => ({
        ...old,
        [key]: serializeSetting(key, value),
      }));
      await Promise.all([
        invalidateSemantic(queryClient),
        key === "semantic.base_url" ? invalidateAi(queryClient) : undefined,
      ]);
    },
  });
}

/** Whether AI features can run, and if not, what is missing (spec §8). */
export function useAiStatus() {
  return useQuery<AiStatus>({
    queryKey: queryKeys.aiStatus,
    queryFn: () => unwrap(commands.getAiStatus()),
    staleTime: Infinity,
  });
}

/** The chat models a provider offers; waits while `provider` is `null`. */
export function useModels(provider: ProviderId | null, enabled = true) {
  return useQuery<ModelInfo[]>({
    queryKey: queryKeys.models(provider),
    queryFn: () => unwrap(commands.listModels(provider ?? "anthropic")),
    staleTime: 10 * 60 * 1000,
    retry: false,
    enabled: enabled && provider !== null,
  });
}

/** What depends on the AI settings, a key or the server address. */
function invalidateAi(queryClient: ReturnType<typeof useQueryClient>) {
  return invalidateKeys(queryClient, [queryKeys.aiStatus, ["models"]]);
}

/** A setting the AI service reads. */
export type AiSettingKey =
  | "ai.provider"
  | "ai.chat_model"
  | "ai.translation_model"
  | "ai.arabic_sources"
  | "ai.monthly_limit_usd"
  | "chat.tafsirs";

/** Stores a setting the AI service reads and refreshes its status. */
export function useSetAiSetting() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, SetSettingVariables<AiSettingKey>>({
    mutationFn: ({ key, value }) => unwrap(commands.setSetting(key, serializeSetting(key, value))),
    onSuccess: async (_data, { key, value }) => {
      queryClient.setQueryData<Partial<Record<string, string>>>(queryKeys.settings, (old) => ({
        ...old,
        [key]: serializeSetting(key, value),
      }));
      await invalidateKeys(queryClient, [queryKeys.aiStatus, ["usage-summary"]]);
    },
  });
}

/** A tafsir summary (`paragraphIdx` null) or paragraph translation. */
export interface TafsirJobVariables {
  /** Tafsir pack id. */
  packId: string;
  /** The passage (`TafsirPassage.id`). */
  tafsirId: number;
  /** The paragraph to translate; `null` for a summary. */
  paragraphIdx: number | null;
  /** Summary of the passage, or translation of one paragraph. */
  mode: TranslationMode;
  /** The interface language to write in: `tr` or `en`. */
  targetLang: string;
}

/** The saved summaries and translations of a passage in a language (spec §9). */
export function useTafsirTranslations(packId: string, tafsirId: number, lang: string) {
  return useQuery<TafsirTranslation[]>({
    queryKey: queryKeys.tafsirTranslations(packId, tafsirId, lang),
    queryFn: () => unwrap(commands.listTafsirTranslations(packId, tafsirId, lang)),
    staleTime: Infinity,
  });
}

/** What a summary or translation is expected to cost; waits while not `enabled`. */
export function useTafsirEstimate(variables: TafsirJobVariables, enabled: boolean) {
  const { packId, tafsirId, paragraphIdx, mode, targetLang } = variables;
  return useQuery<CostEstimate>({
    queryKey: queryKeys.tafsirEstimate(variables),
    queryFn: () =>
      unwrap(commands.estimateTafsirTranslation(packId, tafsirId, paragraphIdx, mode, targetLang)),
    staleTime: 60 * 1000,
    retry: false,
    enabled,
  });
}

/**
 * Summarizes or translates with the tafsir model, streaming the text into
 * `channel`; `requestId` names it for {@link cancelGeneration}. Used through
 * `useStream`, which owns the channel.
 */
export function translateTafsir(
  variables: TafsirJobVariables,
  requestId: string,
  channel: Channel<string>,
): Promise<TranslateResult> {
  const { packId, tafsirId, paragraphIdx, mode, targetLang } = variables;
  return unwrap(
    commands.translateTafsir(requestId, packId, tafsirId, paragraphIdx, mode, targetLang, channel),
  );
}

/** Stops a streamed answer; nothing happens if it already ended. */
export function cancelGeneration(requestId: string): Promise<null> {
  return unwrap(commands.cancelGeneration(requestId));
}

/** Refreshes what a finished tafsir summary or translation changes. */
export function useTafsirTranslated() {
  const queryClient = useQueryClient();
  return (packId: string, tafsirId: number, lang: string) =>
    invalidateKeys(queryClient, [
      queryKeys.tafsirTranslations(packId, tafsirId, lang),
      ["tafsir-estimate"],
      ["usage-summary"],
    ]);
}

/** Every chat session, the most recently used first (spec §10). */
export function useChatSessions() {
  return useQuery<ChatSessionSummary[]>({
    queryKey: queryKeys.chatSessions,
    queryFn: () => unwrap(commands.listChatSessions()),
  });
}

/** A chat session with its messages; waits while `id` is `null`. */
export function useChatSession(id: number | null) {
  return useQuery<ChatSession>({
    queryKey: queryKeys.chatSession(id),
    queryFn: () => unwrap(commands.getChatSession(id ?? 0)),
    enabled: id !== null,
  });
}

/** What sending `message` is expected to cost; waits while not `enabled`. */
export function useChatEstimate(sessionId: number | null, message: string, enabled: boolean) {
  return useQuery<CostEstimate>({
    queryKey: queryKeys.chatEstimate(sessionId, message),
    queryFn: () => unwrap(commands.estimateChat(sessionId, message)),
    staleTime: 60 * 1000,
    retry: false,
    enabled,
    placeholderData: keepPreviousData,
  });
}

/**
 * Sends a chat message and streams the answer into `channel`; `requestId`
 * names it for {@link cancelGeneration}. Used through `useStream`.
 */
export function sendChat(
  variables: { sessionId: number | null; message: string; language: string },
  requestId: string,
  channel: Channel<string>,
): Promise<ChatReply> {
  return unwrap(
    commands.chatSend(
      requestId,
      variables.sessionId,
      variables.message,
      variables.language,
      channel,
    ),
  );
}

/** Refreshes what a chat answer changes. */
export function useChatAnswered() {
  const queryClient = useQueryClient();
  return (sessionId: number | null) =>
    invalidateKeys(queryClient, [
      queryKeys.chatSessions,
      queryKeys.chatSession(sessionId),
      ["chat-estimate"],
      ["usage-summary"],
    ]);
}

/** Deletes a chat session. */
export function useDeleteChatSession() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, number>({
    mutationFn: (id) => unwrap(commands.deleteChatSession(id)),
    onSettled: () => invalidateKeys(queryClient, [queryKeys.chatSessions, ["chat-session"]]),
  });
}

/** Variables of {@link useRenameChatSession}. */
export interface RenameChatVariables {
  /** Session id. */
  id: number;
  /** The new title. */
  title: string;
}

/** Renames a chat session. */
export function useRenameChatSession() {
  const queryClient = useQueryClient();
  return useMutation<null, Error, RenameChatVariables>({
    mutationFn: ({ id, title }) => unwrap(commands.renameChatSession(id, title)),
    onSettled: () => invalidateKeys(queryClient, [queryKeys.chatSessions, ["chat-session"]]),
  });
}

/** Spending on AI in a period, per feature and model (spec §9.4). */
export function useUsageSummary(period: UsagePeriod) {
  return useQuery<UsageSummary>({
    queryKey: queryKeys.usageSummary(period),
    queryFn: () => unwrap(commands.getUsageSummary(period)),
  });
}

/** Most related verses shown per section of the Related tab. */
const RELATED_LIMIT = 10;

/** The verses closest in meaning to a verse; waits while `verseId` is `null`. */
export function useRelatedByMeaning(
  verseId: number | null,
  translationId: string | null,
  enabled = true,
) {
  return useQuery<RelatedVerse[]>({
    queryKey: queryKeys.relatedByMeaning(verseId, translationId),
    queryFn: () => unwrap(commands.relatedByMeaning(verseId ?? 0, translationId, RELATED_LIMIT)),
    staleTime: Infinity,
    enabled: enabled && verseId !== null,
  });
}

/** The verses sharing the most rare roots with a verse; waits while `verseId` is `null`. */
export function useRelatedByRoots(
  verseId: number | null,
  translationId: string | null,
  enabled = true,
) {
  return useQuery<RelatedVerse[]>({
    queryKey: queryKeys.relatedByRoots(verseId, translationId),
    queryFn: () => unwrap(commands.relatedByRoots(verseId ?? 0, translationId, RELATED_LIMIT)),
    staleTime: Infinity,
    enabled: enabled && verseId !== null,
  });
}

/** Fetches the manifest again, ignoring the one-hour cache. */
export function useRefreshCatalog() {
  const queryClient = useQueryClient();
  return useMutation<Catalog>({
    mutationFn: () => unwrap(commands.fetchManifest(true)),
    onSuccess: (catalog) => {
      queryClient.setQueryData(queryKeys.catalog, catalog);
    },
  });
}

/** Variables of {@link useInstallPack} and {@link useInstallBundle}. */
export interface InstallVariables {
  /** Pack or bundle id. */
  id: string;
  /** Confirms that a morphology pack replaces the installed one. */
  confirmReplace?: boolean;
}

function useCatalogMutation<V>(run: (variables: V) => Promise<unknown>) {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, V>({
    mutationFn: run,
    onSettled: () => queryClient.invalidateQueries({ queryKey: queryKeys.catalog }),
  });
}

/** Queues a pack (and what it requires). */
export function useInstallPack() {
  return useCatalogMutation<InstallVariables>(({ id, confirmReplace = false }) =>
    unwrap(commands.installPack(id, confirmReplace)),
  );
}

/** Queues the missing members of a bundle. */
export function useInstallBundle() {
  return useCatalogMutation<InstallVariables>(({ id, confirmReplace = false }) =>
    unwrap(commands.installBundle(id, confirmReplace)),
  );
}

/** Cancels a queued or running job. */
export function useCancelInstall() {
  return useCatalogMutation<string>((id) => unwrap(commands.cancelInstall(id)));
}

/** Variables of {@link useAcknowledgeTerms}. */
export interface AcknowledgeVariables {
  /** Pack id. */
  id: string;
  /** The pack version whose terms were read. */
  version: string;
}

/** Records that the terms of a pack version were read. */
export function useAcknowledgeTerms() {
  return useCatalogMutation<AcknowledgeVariables>(({ id, version }) =>
    unwrap(commands.acknowledgePackTerms(id, version)),
  );
}

/** Removes an installed pack; content queries are refreshed afterwards. */
export function useUninstallPack() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, string>({
    mutationFn: (id) => unwrap(commands.uninstallPack(id)),
    onSettled: () => invalidateContent(queryClient),
  });
}

/** Compacts the content database after removals. */
export function useVacuumContent() {
  return useMutation<VacuumSizes>({
    mutationFn: () => unwrap(commands.vacuumContent()),
  });
}

/** Opens a pack's source page in the default browser. */
export function useOpenPackSource() {
  return useMutation<unknown, Error, string>({
    mutationFn: (id) => unwrap(commands.openPackSource(id)),
  });
}

/** The newest connection log entries (spec §14.1). */
export function useNetworkLog(limit = 100) {
  return useQuery<NetworkLogEntry[]>({
    queryKey: queryKeys.networkLog,
    queryFn: () => unwrap(commands.getNetworkLog(limit)),
    staleTime: 0,
  });
}

/** Everything derived from the user's bookmarks, notes and collections. */
const USER_DATA_KEYS = [["bookmarks"], ["notes"], ["collections"], ["collection"]] as const;

function invalidateKeys(
  queryClient: ReturnType<typeof useQueryClient>,
  keys: readonly (readonly unknown[])[],
) {
  return Promise.all(keys.map((queryKey) => queryClient.invalidateQueries({ queryKey })));
}

/** Every bookmark with its verse, newest first. */
export function useBookmarks(translationIds: readonly string[]) {
  return useQuery<BookmarkedVerse[]>({
    queryKey: ["bookmarks", translationIds],
    queryFn: () => unwrap(commands.listBookmarks([...translationIds])),
  });
}

/** Bookmarks a verse by id, or removes its bookmark; resolves to whether it now has one. */
export function useToggleBookmark() {
  const queryClient = useQueryClient();
  return useMutation<boolean, Error, number>({
    mutationFn: (verseId) => unwrap(commands.toggleBookmark(verseId)),
    onSettled: () => invalidateKeys(queryClient, [["bookmarks"]]),
  });
}

/** The notes touching `range`, or all of them with `null`. */
export function useNotes(range: VerseIdRange | null, enabled = true) {
  return useQuery<Note[]>({
    queryKey: ["notes", range],
    queryFn: () => unwrap(commands.listNotes(range)),
    enabled,
  });
}

/** Creates or replaces a note. */
export function useUpsertNote() {
  const queryClient = useQueryClient();
  return useMutation<Note, Error, NoteInput>({
    mutationFn: (note) => unwrap(commands.upsertNote(note)),
    onSettled: () => invalidateKeys(queryClient, [["notes"], ["collection"]]),
  });
}

/** Deletes a note and its place in any collection. */
export function useDeleteNote() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, number>({
    mutationFn: (id) => unwrap(commands.deleteNote(id)),
    onSettled: () => invalidateKeys(queryClient, [["notes"], ["collections"], ["collection"]]),
  });
}

/** Every collection, the most recently changed first. */
export function useCollections() {
  return useQuery<CollectionSummary[]>({
    queryKey: ["collections"],
    queryFn: () => unwrap(commands.listCollections()),
  });
}

/** A collection with the content of its items. */
export function useCollection(id: number, translationIds: readonly string[]) {
  return useQuery<Collection>({
    queryKey: ["collection", id, translationIds],
    queryFn: () => unwrap(commands.getCollection(id, [...translationIds])),
  });
}

/** Creates or changes a collection. */
export function useUpsertCollection() {
  const queryClient = useQueryClient();
  return useMutation<CollectionSummary, Error, CollectionInput>({
    mutationFn: (collection) => unwrap(commands.upsertCollection(collection)),
    onSettled: () => invalidateKeys(queryClient, [["collections"], ["collection"]]),
  });
}

/** Deletes a collection. */
export function useDeleteCollection() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, number>({
    mutationFn: (id) => unwrap(commands.deleteCollection(id)),
    onSettled: () => invalidateKeys(queryClient, [["collections"]]),
  });
}

/** Adds an item at the end of a collection. */
export function useAddCollectionItem() {
  const queryClient = useQueryClient();
  return useMutation<number, Error, AddCollectionItemVariables>({
    mutationFn: ({ collectionId, item, annotation }) =>
      unwrap(commands.addCollectionItem(collectionId, item, annotation)),
    onSettled: () => invalidateKeys(queryClient, [["collections"], ["collection"]]),
  });
}

/** Changes an item's annotation. */
export function useUpdateCollectionItem() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, { id: number; annotation: string | null }>({
    mutationFn: ({ id, annotation }) => unwrap(commands.updateCollectionItem(id, annotation)),
    onSettled: () => invalidateKeys(queryClient, [["collections"], ["collection"]]),
  });
}

/** Moves an item; the cached order changes at once, so a drag does not jump back. */
export function useMoveCollectionItem() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, MoveCollectionItemVariables>({
    mutationFn: ({ id, toIndex }) => unwrap(commands.moveCollectionItem(id, toIndex)),
    onMutate: async ({ collectionId, id, toIndex }) => {
      const queryKey = ["collection", collectionId];
      await queryClient.cancelQueries({ queryKey });
      queryClient.setQueriesData<Collection>({ queryKey }, (collection) => {
        const moved = collection?.items.find((item) => item.id === id);
        if (!collection || !moved) return collection;
        const items = collection.items.filter((item) => item.id !== id);
        items.splice(toIndex, 0, moved);
        return { ...collection, items };
      });
    },
    onSettled: () => invalidateKeys(queryClient, [["collections"], ["collection"]]),
  });
}

/** Removes an item from its collection. */
export function useRemoveCollectionItem() {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, number>({
    mutationFn: (id) => unwrap(commands.removeCollectionItem(id)),
    onSettled: () => invalidateKeys(queryClient, [["collections"], ["collection"]]),
  });
}

/** The available citation styles. */
export function useCitationStyles() {
  return useQuery<CitationStyle[]>({
    queryKey: ["citation-styles"],
    queryFn: () => unwrap(commands.listCitationStyles()),
    staleTime: Infinity,
  });
}

/** Citation text for a reference, asked for when the user copies it. */
export function fetchCitation(reference: CitationRef, style: string): Promise<string> {
  return unwrap(commands.formatCitation(reference, style));
}

/** Saves a collection as Markdown; resolves to the path, or `null` if cancelled. */
export function useExportCollectionMarkdown() {
  return useMutation<string | null, Error, ExportMarkdownVariables>({
    mutationFn: ({ id, translationIds, labels, style }) =>
      unwrap(commands.exportCollectionMarkdown(id, translationIds, labels, style)),
  });
}

/** Saves the backup; resolves to the path, or `null` if cancelled. */
export function useExportUserData() {
  return useMutation<string | null>({
    mutationFn: () => unwrap(commands.exportUserData()),
  });
}

/** Imports a backup the user picks; resolves to the report, or `null` if cancelled. */
export function useImportUserData() {
  const queryClient = useQueryClient();
  return useMutation<ImportReport | null, Error, ImportMode>({
    mutationFn: (mode) => unwrap(commands.importUserData(mode)),
    onSettled: () => invalidateKeys(queryClient, USER_DATA_KEYS),
  });
}

/** Refreshes everything that depends on the installed packs. */
export function invalidateContent(queryClient: ReturnType<typeof useQueryClient>) {
  return Promise.all(
    [
      queryKeys.catalog,
      queryKeys.translations,
      ["verses"] as const,
      queryKeys.tafsirs,
      ["tafsir"] as const,
      ["tafsir-introduction"] as const,
      ["search"] as const,
      ["compare"] as const,
      queryKeys.morphologyPack,
      ["verse-words"] as const,
      queryKeys.semanticStatus,
      ["search-meaning"] as const,
      ["related-by-meaning"] as const,
      ["related-by-roots"] as const,
      ["word"] as const,
      ["concordance"] as const,
      ["search-roots"] as const,
      ["bookmarks"] as const,
      ["collection"] as const,
      queryKeys.networkLog,
    ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
  );
}
