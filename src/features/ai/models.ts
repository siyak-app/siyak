import type { AiStatus } from "@/lib/bindings";
import { type AiSettingKey, useSetAiSetting } from "@/lib/queries";
import type { AiProvider, Settings } from "@/lib/settings";
import { useUiStore } from "@/stores/ui";

/** Saves an AI setting and mirrors it in the UI store. */
export function useSaveAiSetting() {
  const setPreference = useUiStore((state) => state.setPreference);
  const mutation = useSetAiSetting();
  const save = <K extends AiSettingKey>(key: K, value: Settings[K]) => {
    setPreference(key, value);
    mutation.mutate({ key, value });
  };
  return Object.assign(save, { error: mutation.isError ? mutation.error : null });
}

/** Switches the AI provider; the chosen models belonged to the old one. */
export function chooseProvider(
  save: ReturnType<typeof useSaveAiSetting>,
  current: AiProvider | null,
  next: AiProvider | null,
) {
  if (next === current) return;
  save("ai.provider", next);
  save("ai.chat_model", null);
  save("ai.translation_model", null);
}

/** Whether the provider's key or server address is missing, so no model can be listed. */
export function aiBlocked(status: AiStatus | undefined) {
  return (
    status?.state === "key_missing" ||
    status?.state === "base_url_missing" ||
    status?.state === "base_url_invalid"
  );
}
