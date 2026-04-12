import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UpdateInfo } from "../lib/types";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "error";

// Module-level state so update availability survives view switches
let persistedUpdateInfo: UpdateInfo | null = null;
let persistedStatus: UpdateStatus = "idle";
let persistedError: string | null = null;
let onStateChange: (() => void) | null = null;
let autoCheckDone = false;

const AUTO_CHECK_DELAY_MS = 5_000;
const PERIODIC_CHECK_MS = 4 * 60 * 60 * 1000; // 4 hours

export function useUpdater() {
  const [version, setVersion] = useState<string>("");
  const [status, setStatusLocal] = useState<UpdateStatus>(persistedStatus);
  const [updateInfo, setUpdateInfoLocal] = useState<UpdateInfo | null>(
    persistedUpdateInfo
  );
  const [error, setErrorLocal] = useState<string | null>(persistedError);
  const mountedRef = useRef(true);

  // Sync module-level state to React state
  const syncState = useCallback(() => {
    if (!mountedRef.current) return;
    setStatusLocal(persistedStatus);
    setUpdateInfoLocal(persistedUpdateInfo);
    setErrorLocal(persistedError);
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    onStateChange = syncState;

    // Fetch version on mount
    invoke<string>("get_app_version")
      .then((v) => {
        if (mountedRef.current) setVersion(v);
      })
      .catch(() => {});

    // Auto-check on first mount (app startup)
    if (!autoCheckDone) {
      autoCheckDone = true;
      const timer = setTimeout(() => {
        checkForUpdate();
      }, AUTO_CHECK_DELAY_MS);
      return () => {
        clearTimeout(timer);
        mountedRef.current = false;
        onStateChange = null;
      };
    }

    return () => {
      mountedRef.current = false;
      onStateChange = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Periodic check every 4 hours
  useEffect(() => {
    const interval = setInterval(() => {
      if (
        persistedStatus !== "downloading" &&
        persistedStatus !== "checking"
      ) {
        checkForUpdate();
      }
    }, PERIODIC_CHECK_MS);
    return () => clearInterval(interval);
  }, []);

  const notify = () => {
    onStateChange?.();
  };

  const checkForUpdate = useCallback(async () => {
    persistedStatus = "checking";
    persistedError = null;
    notify();

    try {
      const info = await invoke<UpdateInfo | null>("check_for_update");
      if (info) {
        persistedUpdateInfo = info;
        persistedStatus = "available";
      } else {
        persistedUpdateInfo = null;
        persistedStatus = "up-to-date";
      }
    } catch (e) {
      persistedError =
        e instanceof Error ? e.message : String(e);
      persistedStatus = "error";
    }
    notify();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const installUpdate = useCallback(async () => {
    persistedStatus = "downloading";
    persistedError = null;
    notify();

    try {
      await invoke("install_update");
      // App will restart, so we never reach here
    } catch (e) {
      persistedError =
        e instanceof Error ? e.message : String(e);
      persistedStatus = "error";
      notify();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const dismissUpdate = useCallback(() => {
    persistedStatus = "idle";
    notify();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    version,
    status,
    updateInfo,
    error,
    updateAvailable: persistedStatus === "available",
    checkForUpdate,
    installUpdate,
    dismissUpdate,
  };
}
