import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const POLL_INTERVAL_MS = 500;

export function useOverlayLifecycle(enabled: boolean) {
  const [overlayEligible, setOverlayEligible] = useState(false);
  const mountedRef = useRef(true);
  const enabledRef = useRef(enabled);
  const pollingRef = useRef(false);
  const requestIdRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const poll = useCallback(async () => {
    if (!enabledRef.current || !mountedRef.current || pollingRef.current) return;

    pollingRef.current = true;
    const requestId = ++requestIdRef.current;
    try {
      const eligible = await invoke<boolean>("get_overlay_eligibility");
      if (
        mountedRef.current &&
        enabledRef.current &&
        requestId === requestIdRef.current
      ) {
        setOverlayEligible(Boolean(eligible));
      }
    } catch {
      if (mountedRef.current && requestId === requestIdRef.current) {
        setOverlayEligible(false);
      }
    } finally {
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
      setOverlayEligible(false);
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    void poll();
    timerRef.current = setInterval(() => {
      void poll();
    }, POLL_INTERVAL_MS);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      pollingRef.current = false;
    };
  }, [enabled, poll]);

  return overlayEligible;
}
