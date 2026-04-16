import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ReviewStreamPayload } from "../lib/types";

// Module-level persistent state (survives remount)
let persistedReviewText = "";
let persistedCurrentStream = "";
let persistedIsStreaming = false;
let persistedError: string | null = null;
let persistedMatchId: string | null = null;
let persistedActiveRequestId: string | null = null;

let listenerActive = false;
let onStateChange: (() => void) | null = null;

function ensureListener() {
  if (listenerActive) return;
  listenerActive = true;

  listen<ReviewStreamPayload>("review-stream", (event) => {
    const { kind, text, requestId } = event.payload;

    if (!persistedActiveRequestId || requestId !== persistedActiveRequestId) {
      return;
    }

    switch (kind) {
      case "review-start":
        persistedIsStreaming = true;
        persistedCurrentStream = "";
        persistedError = null;
        persistedReviewText = "";
        break;

      case "review-delta":
        if (text) {
          persistedCurrentStream += text;
        }
        break;

      case "review-end":
        persistedIsStreaming = false;
        if (persistedCurrentStream) {
          persistedReviewText = persistedCurrentStream;
        }
        persistedCurrentStream = "";
        persistedActiveRequestId = null;
        break;

      case "review-cached":
        persistedIsStreaming = false;
        persistedReviewText = text ?? "";
        persistedCurrentStream = "";
        persistedError = null;
        persistedActiveRequestId = null;
        break;

      case "review-error":
        persistedIsStreaming = false;
        persistedError = text ?? "Неизвестная ошибка";
        if (persistedCurrentStream) {
          persistedReviewText = persistedCurrentStream;
        }
        persistedCurrentStream = "";
        persistedActiveRequestId = null;
        break;
    }

    onStateChange?.();
  });
}

export function usePostGameReview() {
  const [, forceUpdate] = useState(0);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    ensureListener();

    onStateChange = () => {
      if (mountedRef.current) {
        forceUpdate((n) => n + 1);
      }
    };

    return () => {
      mountedRef.current = false;
      onStateChange = null;
    };
  }, []);

  const requestReview = useCallback(
    async (matchId: string, puuid: string, forceRefresh = false) => {
      if (persistedIsStreaming) return;

      const requestId = `${matchId}:${Date.now()}:${Math.random().toString(36).slice(2)}`;

      persistedIsStreaming = true;
      persistedError = null;
      persistedCurrentStream = "";
      persistedReviewText = "";
      persistedMatchId = matchId;
      persistedActiveRequestId = requestId;
      forceUpdate((n) => n + 1);

      try {
        await invoke("request_post_game_review", {
          matchId,
          puuid,
          forceRefresh,
          requestId,
        });
      } catch (e) {
        persistedIsStreaming = false;
        persistedError = typeof e === "string" ? e : String(e);
        persistedCurrentStream = "";
        persistedActiveRequestId = null;
        forceUpdate((n) => n + 1);
      }
    },
    []
  );

  const clearReview = useCallback(() => {
    persistedReviewText = "";
    persistedCurrentStream = "";
    persistedIsStreaming = false;
    persistedError = null;
    persistedMatchId = null;
    persistedActiveRequestId = null;
    forceUpdate((n) => n + 1);
  }, []);

  return {
    reviewText: persistedReviewText,
    currentStream: persistedCurrentStream,
    isStreaming: persistedIsStreaming,
    error: persistedError,
    reviewMatchId: persistedMatchId,
    requestReview,
    clearReview,
  };
}
