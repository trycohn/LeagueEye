import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { GoldLaneRow } from "./GoldLaneRow";
import { Coins, X, Loader2 } from "lucide-react";
import type { GoldComparisonData } from "../lib/types";

const POLL_INTERVAL = 4_000;
const OVERLAY_WIDTH = 196;

export function GoldOverlayApp() {
  const [data, setData] = useState<GoldComparisonData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const contentRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(true);
  const dataRef = useRef<GoldComparisonData | null>(null);
  const pollingRef = useRef(false);
  const requestIdRef = useRef(0);

  const fetchData = useCallback(async () => {
    if (!mountedRef.current || pollingRef.current) return;

    pollingRef.current = true;
    const requestId = ++requestIdRef.current;
    const showLoading = dataRef.current === null;

    try {
      if (showLoading) {
        setLoading(true);
      }

      const result = await invoke<GoldComparisonData>("get_gold_comparison");

      if (mountedRef.current && requestId === requestIdRef.current) {
        dataRef.current = result;
        setData(result);
        setError(null);
      }
    } catch (e) {
      if (mountedRef.current && requestId === requestIdRef.current) {
        setError(typeof e === "string" ? e : String(e));
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
    void fetchData();
    const timer = setInterval(() => {
      void fetchData();
    }, POLL_INTERVAL);
    return () => clearInterval(timer);
  }, [fetchData]);

  const updateSize = useCallback(() => {
    if (!contentRef.current) return;
    const h = Math.ceil(contentRef.current.getBoundingClientRect().height);
    invoke("resize_gold_overlay", { width: OVERLAY_WIDTH, height: h }).catch(() => {});
  }, []);

  useEffect(() => {
    const observer = new ResizeObserver(() => updateSize());
    if (contentRef.current) observer.observe(contentRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  useEffect(() => {
    updateSize();
  }, [data, error, loading, updateSize]);

  function handleMouseDown(e: React.MouseEvent) {
    if (!e.shiftKey) return;
    e.preventDefault();
    getCurrentWindow().startDragging();
  }

  function handleClose() {
    getCurrentWindow().hide();
  }

  return (
    <div ref={contentRef} onMouseDown={handleMouseDown} className="select-none">
      <div
        className="rounded-xl border border-accent/30 overflow-hidden"
        style={{ background: "rgba(15, 17, 23, 0.92)" }}
      >
        <div className="flex items-center justify-between px-2.5 py-1 border-b border-border/50">
          <div className="flex items-center gap-1.5">
            <Coins size={13} className="text-text-muted shrink-0 opacity-60" />
            <span className="text-[10px] font-medium text-text-muted opacity-70 tracking-wide uppercase">
              gold
            </span>
          </div>
          <button
            onClick={handleClose}
            className="p-1 rounded hover:bg-bg-hover/50 text-text-muted hover:text-text-primary transition-colors"
          >
            <X size={12} />
          </button>
        </div>

        <div className="px-2.5 py-2">
          {loading && !data && (
            <div className="flex items-center gap-2 py-2 justify-center">
              <Loader2 size={14} className="animate-spin text-gold" />
              <span className="text-xs text-text-muted">Загрузка...</span>
            </div>
          )}

          {error && !data && (
            <p className="text-xs text-loss text-center py-2">{error}</p>
          )}

          {data && data.lanes.length > 0 && (
            <div className="flex flex-col gap-1.5">
              {data.lanes.map((lane) => (
                <GoldLaneRow key={lane.role} lane={lane} />
              ))}
            </div>
          )}

          {data && data.lanes.length === 0 && (
            <p className="text-xs text-text-muted text-center py-2">
              Ожидание данных о ролях...
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
