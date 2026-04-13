import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CoachStreamPayload } from "../lib/types";

// Module-level state survives component unmount/remount
let persistedResult = "";
let persistedStream = "";
let persistedIsStreaming = false;
let persistedError: string | null = null;

// Single global listener for draft-* events
let listenerActive = false;
let onStateChange: (() => void) | null = null;

function ensureListener() {
  if (listenerActive) return;
  listenerActive = true;

  listen<CoachStreamPayload>("coach-stream", (event) => {
    const { kind, text } = event.payload;

    switch (kind) {
      case "draft-start":
        persistedIsStreaming = true;
        persistedStream = "";
        persistedError = null;
        break;

      case "draft-delta":
        if (text) {
          persistedStream += text;
        }
        break;

      case "draft-end":
        persistedIsStreaming = false;
        if (persistedStream) {
          persistedResult = persistedStream;
        }
        persistedStream = "";
        break;

      case "draft-error":
        persistedIsStreaming = false;
        persistedError = text ?? "Неизвестная ошибка";
        persistedStream = "";
        break;
    }

    onStateChange?.();
  });
}

export function useDraftHelper() {
  const [, forceUpdate] = useState(0);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    ensureListener();

    onStateChange = () => {
      if (mountedRef.current) {
        forceUpdate((n: number) => n + 1);
      }
    };

    return () => {
      mountedRef.current = false;
      onStateChange = null;
    };
  }, []);

  const requestDraftAdvice = useCallback(async () => {
    if (persistedIsStreaming) return;

    persistedIsStreaming = true;
    persistedError = null;
    persistedStream = "";
    persistedResult = "";
    forceUpdate((n: number) => n + 1);

    try {
      await invoke("request_draft_advice");
    } catch (e) {
      persistedIsStreaming = false;
      persistedError = typeof e === "string" ? e : String(e);
      persistedStream = "";
      forceUpdate((n: number) => n + 1);
    }
  }, []);

  const clearResult = useCallback(() => {
    persistedResult = "";
    persistedError = null;
    forceUpdate((n: number) => n + 1);
  }, []);

  return {
    result: persistedResult,
    currentStream: persistedStream,
    isStreaming: persistedIsStreaming,
    error: persistedError,
    requestDraftAdvice,
    clearResult,
  };
}
