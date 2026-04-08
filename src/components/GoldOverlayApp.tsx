import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { GoldLaneRow } from "./GoldLaneRow";
import { Coins, X, GripHorizontal, Loader2 } from "lucide-react";
import type { GoldComparisonData } from "../lib/types";
import {
  goldOverlayWidth,
  parseGoldOverlayLayout,
  type GoldOverlayLayout,
} from "../lib/goldOverlayLayout";
import { GOLD_OVERLAY_MOCK_DATA } from "../lib/goldOverlayMockData";

const POLL_INTERVAL = 10_000;

function isGoldOverlayBrowserMock(): boolean {
  if (!import.meta.env.DEV) return false;
  return new URLSearchParams(window.location.search).has("mock");
}

function useGoldOverlayLayout(): GoldOverlayLayout {
  return useMemo(
    () => parseGoldOverlayLayout(window.location.search),
    []
  );
}

export function GoldOverlayApp() {
  const layout = useGoldOverlayLayout();
  const [data, setData] = useState<GoldComparisonData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const contentRef = useRef<HTMLDivElement>(null);

  const fetchData = useCallback(async () => {
    if (isGoldOverlayBrowserMock()) {
      setData(GOLD_OVERLAY_MOCK_DATA);
      setError(null);
      setLoading(false);
      return;
    }
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

  useEffect(() => {
    fetchData();
    if (isGoldOverlayBrowserMock()) return;
    const timer = setInterval(fetchData, POLL_INTERVAL);
    return () => clearInterval(timer);
  }, [fetchData]);

  const overlayWidth = goldOverlayWidth(layout);

  const updateSize = useCallback(() => {
    if (!contentRef.current) return;
    const h = Math.ceil(contentRef.current.getBoundingClientRect().height);
    if (isGoldOverlayBrowserMock()) return;
    invoke("resize_gold_overlay", { width: overlayWidth, height: h }).catch(() => {});
  }, [overlayWidth]);

  useEffect(() => {
    const observer = new ResizeObserver(() => updateSize());
    if (contentRef.current) observer.observe(contentRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  useEffect(() => {
    updateSize();
  }, [data, error, loading, updateSize, layout]);

  function handleMouseDown(e: React.MouseEvent) {
    if (!e.shiftKey || isGoldOverlayBrowserMock()) return;
    e.preventDefault();
    getCurrentWindow().startDragging();
  }

  function handleClose() {
    if (isGoldOverlayBrowserMock()) return;
    getCurrentWindow().hide();
  }

  const headerPad =
    layout === "classic"
      ? "px-3 py-1.5"
      : layout === "compact"
        ? "px-2.5 py-1"
        : layout === "single"
          ? "px-2 py-1"
          : "px-2 py-0.5";

  const contentPad =
    layout === "classic"
      ? "px-3 py-3"
      : layout === "compact"
        ? "px-2.5 py-2"
        : layout === "single"
          ? "px-2 py-1.5"
          : "px-2 py-1";

  const rowGap =
    layout === "classic"
      ? "gap-2.5"
      : layout === "compact"
        ? "gap-1.5"
        : layout === "single"
          ? "gap-1"
          : "gap-0.5";

  const showHint = layout === "classic" || layout === "compact";

  return (
    <div ref={contentRef} onMouseDown={handleMouseDown} className="select-none">
      <div
        className="rounded-xl border border-accent/30 overflow-hidden"
        style={{ background: "rgba(15, 17, 23, 0.92)" }}
      >
        <div
          className={`flex items-center justify-between border-b border-border/50 ${headerPad}`}
        >
          <div className="flex items-center gap-1.5 min-w-0">
            <Coins
              size={layout === "micro" ? 11 : layout === "single" ? 12 : 14}
              className="text-gold shrink-0"
            />
            <span
              className={`font-bold text-gold truncate ${
                layout === "micro" ? "text-[10px]" : "text-xs"
              }`}
            >
              Золото
            </span>
          </div>
          <div className="flex items-center gap-1 shrink-0">
            {showHint && (
              <>
                <GripHorizontal size={12} className="text-text-muted" />
                <span className="text-[10px] text-text-muted whitespace-nowrap">
                  Shift+drag
                </span>
              </>
            )}
            <button
              onClick={handleClose}
              className="p-1 rounded hover:bg-bg-hover/50 text-text-muted hover:text-text-primary transition-colors"
            >
              <X size={layout === "micro" ? 11 : 12} />
            </button>
          </div>
        </div>

        <div className={contentPad}>
          {loading && !data && (
            <div className="flex items-center gap-2 py-2 justify-center">
              <Loader2
                size={layout === "micro" ? 12 : 14}
                className="animate-spin text-gold"
              />
              <span className="text-xs text-text-muted">Загрузка...</span>
            </div>
          )}

          {error && !data && (
            <p className="text-xs text-loss text-center py-2">{error}</p>
          )}

          {data && data.lanes.length > 0 && (
            <div className={`flex flex-col ${rowGap}`}>
              {data.lanes.map((lane) => (
                <GoldLaneRow key={lane.role} lane={lane} layout={layout} />
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
