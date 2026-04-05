import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CoachStreamPayload, CoachMessage } from "../lib/types";

// Module-level state survives component unmount/remount
// (LiveGameView remounts on every live poll phase change)
let persistedMessages: CoachMessage[] = [];
let persistedStream = "";
let persistedIsStreaming = false;
let persistedError: string | null = null;

// Single global listener — avoid duplicate listeners on remount
let listenerActive = false;
let onStateChange: (() => void) | null = null;

function ensureListener() {
  if (listenerActive) return;
  listenerActive = true;

  listen<CoachStreamPayload>("coach-stream", (event) => {
    const { kind, text } = event.payload;

    switch (kind) {
      case "start":
        persistedIsStreaming = true;
        persistedStream = "";
        persistedError = null;
        break;

      case "delta":
        if (text) {
          persistedStream += text;
        }
        break;

      case "end":
        persistedIsStreaming = false;
        if (persistedStream) {
          persistedMessages = [
            ...persistedMessages,
            { text: persistedStream, timestamp: Date.now() },
          ];
        }
        persistedStream = "";
        break;

      case "error":
        persistedIsStreaming = false;
        persistedError = text ?? "Неизвестная ошибка";
        persistedStream = "";
        break;
    }

    // Notify the currently mounted component to re-render
    onStateChange?.();
  });
}

export function useAiCoach() {
  const [, forceUpdate] = useState(0);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    ensureListener();

    // Register this component as the state change listener
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

  const requestAdvice = useCallback(async () => {
    if (persistedIsStreaming) return;
    persistedError = null;
    forceUpdate((n: number) => n + 1);
    try {
      await invoke("request_coaching");
    } catch (e) {
      persistedError = typeof e === "string" ? e : String(e);
      forceUpdate((n: number) => n + 1);
    }
  }, []);

  const clearMessages = useCallback(() => {
    persistedMessages = [];
    persistedError = null;
    forceUpdate((n: number) => n + 1);
  }, []);

  return {
    messages: persistedMessages,
    currentStream: persistedStream,
    isStreaming: persistedIsStreaming,
    error: persistedError,
    requestAdvice,
    clearMessages,
  };
}
