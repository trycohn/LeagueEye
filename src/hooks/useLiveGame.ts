import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LiveGameData } from "../lib/types";

const CHAMP_SELECT_INTERVAL = 3_000;
const IN_GAME_INTERVAL = 15_000;
const IDLE_INTERVAL = 8_000;

export function useLiveGame(enabled: boolean) {
  const [liveData, setLiveData] = useState<LiveGameData | null>(null);
  const [loading, setLoading] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  const phase = liveData?.phase ?? "none";

  const poll = useCallback(async () => {
    if (!enabled || !mountedRef.current) return;
    try {
      setLoading(true);
      const data = await invoke<LiveGameData>("get_live_game");
      if (mountedRef.current) setLiveData(data);
    } catch {
      if (mountedRef.current) setLiveData(null);
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, [enabled]);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  useEffect(() => {
    if (!enabled) {
      setLiveData(null);
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

    poll();

    timerRef.current = setInterval(poll, interval);
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [enabled, phase, poll]);

  return { liveData, phase, loading };
}
