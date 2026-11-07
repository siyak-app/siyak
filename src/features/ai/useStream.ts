import { Channel } from "@tauri-apps/api/core";
import { useCallback, useRef, useState } from "react";

import { cancelGeneration } from "@/lib/queries";

/** A streamed answer in progress: its text so far. */
export interface StreamState<K> {
  /** What is being written, e.g. which paragraph. */
  key: K;
  text: string;
}

/**
 * Runs one streamed AI request at a time (tafsir summary, translation or a
 * chat answer): `start` hands the request a fresh id and a channel whose
 * pieces are appended to `streaming.text`; `stop` cancels it in the core.
 */
export function useStream<K>() {
  const [streaming, setStreaming] = useState<StreamState<K> | null>(null);
  const requestId = useRef<string | null>(null);

  const start = useCallback(
    async <T>(key: K, run: (requestId: string, channel: Channel<string>) => Promise<T>) => {
      const id = crypto.randomUUID();
      requestId.current = id;
      const channel = new Channel<string>();
      channel.onmessage = (piece) => {
        if (requestId.current !== id) return;
        setStreaming((state) => (state ? { ...state, text: state.text + piece } : state));
      };
      setStreaming({ key, text: "" });
      try {
        return await run(id, channel);
      } finally {
        if (requestId.current === id) {
          requestId.current = null;
          setStreaming(null);
        }
      }
    },
    [],
  );

  const stop = useCallback(() => {
    const id = requestId.current;
    if (id) void cancelGeneration(id);
  }, []);

  return { streaming, start, stop };
}
