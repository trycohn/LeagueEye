import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { GoldLaneRow } from "./GoldLaneRow";
import { Coins, X, GripHorizontal, Loader2 } from "lucide-react";
import type { GoldComparisonData } from "../lib/types";

const POLL_INTERVAL = 10_000;

export function GoldOverlayApp() {
  const [data, setData] = useState<GoldComparisonData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const contentRef = useRef<HTMLDivElement>(null);

  const fetchData = useCallback(async () => {
    try {
      const result = await invoke<GoldComparisonData>("get_gold_comparison");
      setData(result);
      setError(null);
    } catch (e) {
      setError(typeof e === "string" ? e : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Poll on mount + interval
  useEffect(() => {
    fetchData();
    const timer = setInterval(fetchData, POLL_INTERVAL);
    return () => clearInterval(timer);
  }, [fetchData]);

  // Auto-resize
  const updateSize = useCallback(() => {
    if (!contentRef.current) return;
    const h = Math.ceil(contentRef.current.getBoundingClientRect().height);
    invoke("resize_gold_overlay", { width: 340, height: h }).catch(() => {});
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
    if (e.shiftKey) {
      e.preventDefault();
      getCurrentWindow().startDragging();
    }
  }

  function handleClose() {
    getCurrentWindow().hide();
  }

  return (
    <div ref={contentRef} onMouseDown={handleMouseDown} className="select-none">
      <div className="rounded-xl border border-accent/30 overflow-hidden"
           style={{ background: "rgba(15, 17, 23, 0.92)" }}>
        {/* Header */}
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-border/50">
          <div className="flex items-center gap-2">
            <Coins size={14} className="text-gold" />
            <span className="text-xs font-bold text-gold">Золото</span>
          </div>
          <div className="flex items-center gap-1.5">
            <GripHorizontal size={12} className="text-text-muted" />
            <span className="text-[10px] text-text-muted">Shift+drag</span>
            <button
              onClick={handleClose}
              className="ml-2 p-1 rounded hover:bg-bg-hover/50 text-text-muted hover:text-text-primary transition-colors"
            >
              <X size={12} />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="px-3 py-2">
          {loading && !data && (
            <div className="flex items-center gap-2 py-3 justify-center">
              <Loader2 size={14} className="animate-spin text-gold" />
              <span className="text-xs text-text-muted">Загрузка...</span>
            </div>
          )}

          {error && !data && (
            <p className="text-xs text-loss text-center py-3">{error}</p>
          )}

          {data && data.lanes.length > 0 && (
            <div className="flex flex-col gap-1.5">
              {data.lanes.map((lane) => (
                <GoldLaneRow key={lane.role} lane={lane} />
              ))}
            </div>
          )}

          {data && data.lanes.length === 0 && (
            <p className="text-xs text-text-muted text-center py-3">
              Ожидание данных о ролях...
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
