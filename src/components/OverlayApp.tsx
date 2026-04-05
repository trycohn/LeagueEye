import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CoachText } from "./CoachText";
import { useAiCoach } from "../hooks/useAiCoach";
import { Sparkles, Loader2, X, GripHorizontal } from "lucide-react";

export function OverlayApp() {
  const { messages, currentStream, isStreaming, error, requestAdvice } = useAiCoach();
  const mountedRef = useRef(false);
  const contentRef = useRef<HTMLDivElement>(null);

  // Auto-resize window to fit content
  const updateSize = useCallback(() => {
    if (!contentRef.current) return;
    const rect = contentRef.current.getBoundingClientRect();
    const h = Math.ceil(rect.height);
    invoke("resize_overlay", { width: 420, height: h }).catch(() => {});
  }, []);

  useEffect(() => {
    const observer = new ResizeObserver(() => updateSize());
    if (contentRef.current) observer.observe(contentRef.current);
    return () => observer.disconnect();
  }, [updateSize]);

  // Also update after stream changes
  useEffect(() => {
    updateSize();
  }, [currentStream, messages, isStreaming, error, updateSize]);

  // Auto-request on first mount
  useEffect(() => {
    if (!mountedRef.current) {
      mountedRef.current = true;
      requestAdvice();
    }
  }, [requestAdvice]);

  // Listen for hotkey re-trigger
  useEffect(() => {
    const unlisten = listen("hotkey-coach-trigger", () => {
      requestAdvice();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [requestAdvice]);

  const latestMessage = messages.length > 0 ? messages[messages.length - 1] : null;

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
        <div className="flex items-center justify-between px-3 py-2 border-b border-border/50">
          <div className="flex items-center gap-2">
            <Sparkles size={14} className="text-accent" />
            <span className="text-xs font-bold text-accent">AI Coach</span>
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
        <div className="p-3">
          {error && (
            <p className="text-xs text-loss mb-2">{error}</p>
          )}

          {isStreaming && currentStream && (
            <div>
              <CoachText text={currentStream} />
              <span className="inline-block w-0.5 h-3 bg-accent animate-pulse ml-0.5 align-text-bottom" />
            </div>
          )}

          {isStreaming && !currentStream && (
            <div className="flex items-center gap-2 py-3 justify-center">
              <Loader2 size={14} className="animate-spin text-accent" />
              <span className="text-xs text-text-muted">Анализирую...</span>
            </div>
          )}

          {!isStreaming && latestMessage && (
            <CoachText text={latestMessage.text} />
          )}

          {!isStreaming && !latestMessage && !error && (
            <p className="text-xs text-text-muted text-center py-3">
              Shift+E — получить совет
            </p>
          )}
        </div>

        {/* Footer */}
        <div className="px-3 py-1.5 border-t border-border/30 flex items-center justify-between">
          <span className="text-[10px] text-text-muted">Shift+E — новый совет</span>
          {!isStreaming && (
            <button
              onClick={() => requestAdvice()}
              className="text-[10px] text-accent hover:text-accent-hover transition-colors"
            >
              Обновить
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
