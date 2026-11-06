import type { ApiProvider } from "@/lib/bindings";

/** Names of the embedding providers, as their makers write them. */
export const PROVIDER_NAMES: Record<ApiProvider, string> = {
  openai: "OpenAI",
  voyage: "Voyage AI",
  gemini: "Google Gemini",
  openai_compatible: "OpenAI-compatible server",
};
