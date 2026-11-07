import type { ProviderId } from "@/lib/bindings";

/** Names of the API providers, as their makers write them. */
export const PROVIDER_NAMES: Record<ProviderId, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  voyage: "Voyage AI",
  gemini: "Google Gemini",
  openai_compatible: "OpenAI-compatible server",
};
