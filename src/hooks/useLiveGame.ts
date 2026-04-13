import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LiveGameData } from "../lib/types";

const CHAMP_SELECT_INTERVAL = 2_000;
const IN_GAME_INTERVAL = 5_000;
const IDLE_INTERVAL = 3_000;

export function useLiveGame(enabled: boolean) {
  const [liveData, setLiveData] = useState<LiveGameData | null>(null);
  const [loading, setLoading] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const enabledRef = useRef(enabled);
  const liveDataRef = useRef<LiveGameData | null>(null);
  const pollingRef = useRef(false);
  const requestIdRef = useRef(0);

  const phase = liveData?.phase ?? "none";

  const poll = useCallback(async () => {
    if (!enabledRef.current || !mountedRef.current || pollingRef.current) return;

    pollingRef.current = true;
    const requestId = ++requestIdRef.current;
    const showLoading = liveDataRef.current === null;

    try {
      if (showLoading) {
        setLoading(true);
      }

      const data = await invoke<LiveGameData>("get_live_game");

      if (
        mountedRef.current &&
        enabledRef.current &&
        requestId === requestIdRef.current
      ) {
        liveDataRef.current = data;
        setLiveData(data);
      }
    } catch {
      if (
        mountedRef.current &&
        enabledRef.current &&
        requestId === requestIdRef.current &&
        liveDataRef.current === null
      ) {
        setLiveData(null);
      }
    } finally {
      if (mountedRef.current && requestId === requestIdRef.current && showLoading) {
        setLoading(false);
      }
      pollingRef.current = false;
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    enabledRef.current = enabled;

    if (!enabled) {
      requestIdRef.current += 1;
      liveDataRef.current = null;
      setLiveData(null);
      setLoading(false);
      return;
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    let interval: number;
    if (phase === "champ_select") {
      interval = CHAMP_SELECT_INTERVAL;
    } else if (phase === "in_game") {
      interval = IN_GAME_INTERVAL;
    } else {
      interval = IDLE_INTERVAL;
    }

    void poll();

    timerRef.current = setInterval(() => {
      void poll();
    }, interval);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [enabled, phase, poll]);

  return { liveData, phase, loading };
}
