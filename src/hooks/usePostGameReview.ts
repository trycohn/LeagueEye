import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PostGameReview, ReviewStreamPayload } from "../lib/types";

const PENDING_MESSAGE = "Разбор уже генерируется для этого игрока";

let persistedReviewText = "";
let persistedCurrentStream = "";
let persistedIsStreaming = false;
let persistedIsPending = false;
let persistedPendingMessage: string | null = null;
let persistedError: string | null = null;
let persistedMatchId: string | null = null;
let persistedPuuid: string | null = null;
let persistedActiveRequestId: string | null = null;

let listenerActive = false;
let onStateChange: (() => void) | null = null;
let pendingPollTimer: number | null = null;
let pendingPollKey: string | null = null;

function makeReviewKey(matchId: string, puuid: string) {
  return `${matchId}:${puuid}`;
}

function getPersistedReviewKey() {
  if (!persistedMatchId || !persistedPuuid) {
    return null;
  }
  return makeReviewKey(persistedMatchId, persistedPuuid);
}

function notifyStateChange() {
  onStateChange?.();
}

function stopPendingPoll() {
  if (pendingPollTimer !== null) {
    window.clearTimeout(pendingPollTimer);
    pendingPollTimer = null;
  }
  pendingPollKey = null;
}

function hydrateReviewState(review: PostGameReview) {
  persistedMatchId = review.matchId;
  persistedPuuid = review.puuid;
  persistedReviewText = review.reviewText ?? "";
  persistedCurrentStream = "";
  persistedActiveRequestId = null;

  if (review.status === "ready") {
    persistedIsStreaming = false;
    persistedIsPending = false;
    persistedPendingMessage = null;
    persistedError = null;
    stopPendingPoll();
    return;
  }

  if (review.status === "generating") {
    persistedIsStreaming = false;
    persistedIsPending = true;
    persistedPendingMessage = PENDING_MESSAGE;
    persistedError = null;
    return;
  }

  persistedIsStreaming = false;
  persistedIsPending = false;
  persistedPendingMessage = null;
  persistedError = review.errorText ?? "Предыдущий разбор завершился с ошибкой";
  stopPendingPoll();
}

async function fetchCachedReview(matchId: string, puuid: string) {
  return invoke<PostGameReview | null>("get_cached_review", { matchId, puuid });
}

async function pollPendingReview(matchId: string, puuid: string) {
  const reviewKey = makeReviewKey(matchId, puuid);
  if (pendingPollKey !== reviewKey) {
    return;
  }

  try {
    const review = await fetchCachedReview(matchId, puuid);
    if (pendingPollKey !== reviewKey) {
      return;
    }

    if (!review) {
      persistedIsPending = false;
      persistedPendingMessage = null;
      notifyStateChange();
      stopPendingPoll();
      return;
    }

    hydrateReviewState(review);
    notifyStateChange();

    if (review.status === "generating") {
      pendingPollKey = reviewKey;
      pendingPollTimer = window.setTimeout(() => {
        void pollPendingReview(matchId, puuid);
      }, 2000);
      return;
    }

    stopPendingPoll();
  } catch {
    if (pendingPollKey !== reviewKey) {
      return;
    }
    pendingPollTimer = window.setTimeout(() => {
      void pollPendingReview(matchId, puuid);
    }, 3000);
  }
}

function startPendingPoll(matchId: string, puuid: string) {
  const reviewKey = makeReviewKey(matchId, puuid);
  if (pendingPollKey === reviewKey && pendingPollTimer !== null) {
    return;
  }

  stopPendingPoll();
  pendingPollKey = reviewKey;
  pendingPollTimer = window.setTimeout(() => {
    void pollPendingReview(matchId, puuid);
  }, 2000);
}

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
        stopPendingPoll();
        persistedIsStreaming = true;
        persistedIsPending = false;
        persistedPendingMessage = null;
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
        stopPendingPoll();
        persistedIsStreaming = false;
        persistedIsPending = false;
        persistedPendingMessage = null;
        if (persistedCurrentStream) {
          persistedReviewText = persistedCurrentStream;
        }
        persistedCurrentStream = "";
        persistedActiveRequestId = null;
        break;

      case "review-cached":
        stopPendingPoll();
        persistedIsStreaming = false;
        persistedIsPending = false;
        persistedPendingMessage = null;
        persistedReviewText = text ?? "";
        persistedCurrentStream = "";
        persistedError = null;
        persistedActiveRequestId = null;
        break;

      case "review-pending":
        persistedIsStreaming = false;
        persistedIsPending = true;
        persistedPendingMessage = text ?? PENDING_MESSAGE;
        persistedCurrentStream = "";
        persistedError = null;
        persistedActiveRequestId = null;
        if (persistedMatchId && persistedPuuid) {
          startPendingPoll(persistedMatchId, persistedPuuid);
        }
        break;

      case "review-error":
        stopPendingPoll();
        persistedIsStreaming = false;
        persistedIsPending = false;
        persistedPendingMessage = null;
        persistedError = text ?? "Неизвестная ошибка";
        if (persistedCurrentStream) {
          persistedReviewText = persistedCurrentStream;
        }
        persistedCurrentStream = "";
        persistedActiveRequestId = null;
        break;
    }

    notifyStateChange();
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
      const reviewKey = makeReviewKey(matchId, puuid);
      if (
        !forceRefresh &&
        getPersistedReviewKey() === reviewKey &&
        (persistedIsStreaming || persistedIsPending)
      ) {
        return;
      }

      const requestId = `${reviewKey}:${Date.now()}:${Math.random().toString(36).slice(2)}`;

      stopPendingPoll();
      persistedIsStreaming = true;
      persistedIsPending = false;
      persistedPendingMessage = null;
      persistedError = null;
      persistedCurrentStream = "";
      persistedReviewText = "";
      persistedMatchId = matchId;
      persistedPuuid = puuid;
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

  const ensureReview = useCallback(
    async (matchId: string, puuid: string) => {
      const reviewKey = makeReviewKey(matchId, puuid);

      if (
        getPersistedReviewKey() === reviewKey &&
        (persistedIsStreaming || persistedIsPending || !!persistedReviewText || !!persistedError)
      ) {
        if (persistedIsPending) {
          startPendingPoll(matchId, puuid);
        }
        return;
      }

      stopPendingPoll();

      try {
        const cachedReview = await fetchCachedReview(matchId, puuid);
        if (cachedReview) {
          hydrateReviewState(cachedReview);
          forceUpdate((n) => n + 1);

          if (cachedReview.status === "generating") {
            startPendingPoll(matchId, puuid);
          }
          return;
        }
      } catch {
        // Ignore cache lookup failures and fall back to a fresh request.
      }

      await requestReview(matchId, puuid);
    },
    [requestReview]
  );

  const clearReview = useCallback(() => {
    stopPendingPoll();
    persistedReviewText = "";
    persistedCurrentStream = "";
    persistedIsStreaming = false;
    persistedIsPending = false;
    persistedPendingMessage = null;
    persistedError = null;
    persistedMatchId = null;
    persistedPuuid = null;
    persistedActiveRequestId = null;
    forceUpdate((n) => n + 1);
  }, []);

  return {
    reviewText: persistedReviewText,
    currentStream: persistedCurrentStream,
    isStreaming: persistedIsStreaming,
    isPending: persistedIsPending,
    pendingMessage: persistedPendingMessage,
    error: persistedError,
    reviewMatchId: persistedMatchId,
    reviewPuuid: persistedPuuid,
    ensureReview,
    requestReview,
    clearReview,
  };
}
