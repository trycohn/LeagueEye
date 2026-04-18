import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Flag, Loader2, X } from "lucide-react";
import { objectiveIconUrl } from "../lib/ddragon";
import type { ObjectiveKind, ObjectiveOverlayData } from "../lib/types";

const POLL_INTERVAL = 4_000;
const OVERLAY_WIDTH = 420;
const OBJECTIVE_LABELS: Record<ObjectiveKind, string> = {
  tower: "Башни",
  dragon: "Драконы",
  herald: "Герольд",
  baron: "Барон",
  inhibitor: "Ингибы",
};

export function ObjectiveOverlayApp() {
  const [data, setData] = useState<ObjectiveOverlayData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const contentRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(true);
  const dataRef = useRef<ObjectiveOverlayData | null>(null);
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

      const result = await invoke<ObjectiveOverlayData>("get_objective_summary");

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
    const height = Math.ceil(contentRef.current.getBoundingClientRect().height);
    invoke("resize_objective_overlay", { width: OVERLAY_WIDTH, height }).catch(() => {});
  }, []);

  useEffect(() => {
    const observer = new ResizeObserver(() => updateSize());
    if (contentRef.current) observer.observe(contentRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  useEffect(() => {
    updateSize();
  }, [data, error, loading, updateSize]);

  async function handleMouseDown(e: React.MouseEvent) {
    const target = e.target as HTMLElement;
    if (!e.shiftKey || target.closest("button")) return;
    e.preventDefault();
    await getCurrentWindow().startDragging();
  }

  function handleClose() {
    getCurrentWindow().hide();
  }

  const metrics = data?.objectives ?? [];

  return (
    <div ref={contentRef} onMouseDown={handleMouseDown} className="select-none">
      <div
        className="rounded-xl border border-accent/30 overflow-hidden"
        style={{ background: "rgba(15, 17, 23, 0.92)" }}
      >
        <div className="flex items-center justify-between px-2.5 py-1 border-b border-border/50">
          <div className="flex items-center gap-1.5">
            <Flag size={13} className="text-text-muted shrink-0 opacity-65" />
            <span className="text-[10px] font-medium text-text-muted opacity-70 tracking-wide uppercase">
              objectives
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[9px] text-text-muted/80 uppercase tracking-wide">мы : враги</span>
            <button
              onClick={handleClose}
              className="p-1 rounded hover:bg-bg-hover/50 text-text-muted hover:text-text-primary transition-colors"
            >
              <X size={12} />
            </button>
          </div>
        </div>

        <div className="px-2.5 py-2">
          {loading && !data && (
            <div className="flex items-center gap-2 py-2 justify-center">
              <Loader2 size={14} className="animate-spin text-accent" />
              <span className="text-xs text-text-muted">Загрузка...</span>
            </div>
          )}

          {error && !data && (
            <p className="text-xs text-loss text-center py-2">{error}</p>
          )}

          {data && (
            <div className="flex flex-col gap-1.5">
              <div className="grid grid-cols-5 gap-1.5">
                {metrics.map((metric) => (
                  <div
                    key={metric.kind}
                    title={OBJECTIVE_LABELS[metric.kind]}
                    className="flex items-center justify-center gap-1 rounded-lg border border-border/60 bg-bg-secondary/40 px-1.5 py-1.5 min-w-0"
                  >
                    <img
                      src={objectiveIconUrl(metric.kind)}
                      alt={OBJECTIVE_LABELS[metric.kind]}
                      className="w-4 h-4 shrink-0 opacity-90"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = "none";
                      }}
                    />
                    <div className="flex items-center gap-0.5 text-[10px] font-semibold tabular-nums leading-none">
                      <span className="text-win">{metric.allyCount}</span>
                      <span className="text-text-muted/80">:</span>
                      <span className="text-loss">{metric.enemyCount}</span>
                    </div>
                  </div>
                ))}
              </div>

              <div className="rounded-lg border border-border/40 bg-bg-secondary/25 px-2 py-1">
                <p
                  className="text-[10px] leading-tight text-text-secondary truncate"
                  title={data.lastEvent?.text ?? "Ожидание первого объекта..."}
                >
                  {data.lastEvent?.text ?? "Ожидание первого объекта..."}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
