# 0013. Talk to AI providers over plain HTTP and server-sent events

- Status: Accepted
- Date: 2026-11-07

## Context

Chat and tafsir summaries (spec §8–§10) need four providers: Anthropic, OpenAI, Google Gemini and
any OpenAI-compatible server (Ollama, LM Studio, OpenRouter). Every request must go through the
one allow-listed, logged HTTP client (`net.rs`, spec §14.1), keys must come from the keychain for
each request, and answers must stream so a long translation shows as it is written. Rust SDKs for
these providers are unofficial or partial, each brings its own HTTP client, and none would use
`net.rs`.

## Decision

Each provider is a small module (`llm/anthropic.rs`, `llm/openai.rs`, `llm/gemini.rs`) that builds
its JSON request by hand, sends it with `Net::post_stream` and reads the answer as server-sent
events (`llm/sse.rs`). They share one trait, `LlmProvider`: list the models, and stream an answer
into a text sink until it ends or is cancelled, returning the provider's token counts. The
OpenAI module also serves OpenAI-compatible servers at the address the user entered. No provider
SDK is a dependency.

## Consequences

- Every AI request is on the allow-list (or the user's own server), refuses redirects, and is in
  the connection log with purpose `chat` or `translate`; keys never leave Rust.
- Only the parts of each API the app uses are implemented: text in, text out, usage. Tools,
  images and prompt caching are not, and need code when they are wanted.
- Provider API changes are ours to follow. Each provider is tested against a local server that
  streams in its format (`llm/http_tests.rs`); a change in a real API shows up as a failing
  request with `provider_failed` and the provider's message in the diagnostic.
- Thinking models send nothing, not even the response headers, until they finish thinking. Model
  streams therefore wait up to 5 minutes for data (other requests: 30 seconds), can be stopped
  before the answer starts, and a timeout is reported as `provider_timeout`, not as a network
  failure. Gemini 3 and later are asked to think `low`; a 503 is retried once after 2 seconds and
  then reported as `provider_overloaded`.
