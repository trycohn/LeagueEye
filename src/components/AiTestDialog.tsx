// TEST: удалить этот файл после тестирования стриминга AI
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CoachStreamPayload } from "../lib/types";
import { Sparkles, Send, Loader2, X, Trash2 } from "lucide-react";

interface StreamEntry {
  text: string;
  ttfb: number | null;
  totalMs: number;
  timestamp: number;
}

export function AiTestDialog({ onClose }: { onClose: () => void }) {
  const [input, setInput] = useState("");
  const [stream, setStream] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<StreamEntry[]>([]);
  const [ttfb, setTtfb] = useState<number | null>(null);
  const [elapsed, setElapsed] = useState(0);

  const startRef = useRef(0);
  const firstTokenRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const streamRef = useRef("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<CoachStreamPayload>("coach-stream", (event) => {
      const { kind, text } = event.payload;
      switch (kind) {
        case "start":
          startRef.current = Date.now();
          firstTokenRef.current = 0;
          streamRef.current = "";
          setStream("");
          setStreaming(true);
          setError(null);
          setTtfb(null);
          timerRef.current = setInterval(() => {
            setElapsed(Date.now() - startRef.current);
          }, 100);
          break;
        case "delta":
          if (text) {
            if (!firstTokenRef.current) {
              firstTokenRef.current = Date.now();
              setTtfb(firstTokenRef.current - startRef.current);
            }
            streamRef.current += text;
            setStream(streamRef.current);
          }
          break;
        case "end": {
          const totalMs = Date.now() - startRef.current;
          if (timerRef.current) clearInterval(timerRef.current);
          setStreaming(false);
          setElapsed(totalMs);
          if (streamRef.current) {
            setHistory((h) => [
              {
                text: streamRef.current,
                ttfb: firstTokenRef.current
                  ? firstTokenRef.current - startRef.current
                  : null,
                totalMs,
                timestamp: Date.now(),
              },
              ...h,
            ]);
          }
          streamRef.current = "";
          setStream("");
          break;
        }
        case "error":
          if (timerRef.current) clearInterval(timerRef.current);
          setStreaming(false);
          setError(text ?? "Неизвестная ошибка");
          streamRef.current = "";
          setStream("");
          break;
      }
    });
    return () => {
      unlisten.then((fn) => fn());
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, []);

  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [stream]);

  async function handleSend() {
    if (streaming || !input.trim()) return;
    setError(null);
    try {
      await invoke("test_coaching", { message: input.trim() });
    } catch (e) {
      setError(typeof e === "string" ? e : String(e));
    }
  }

  function formatMs(ms: number) {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-2xl mx-4 rounded-2xl bg-bg-card border border-border shadow-2xl flex flex-col max-h-[85vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <div className="flex items-center gap-2">
            <Sparkles size={18} className="text-accent" />
            <h2 className="text-sm font-bold text-accent">
              AI Streaming Test
            </h2>
            <span className="text-[10px] text-text-muted bg-loss/20 text-loss px-1.5 py-0.5 rounded font-mono">
              TEST
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-bg-hover text-text-muted hover:text-text-primary transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        {/* Metrics */}
        {(streaming || elapsed > 0) && (
          <div className="flex items-center gap-4 px-5 py-2 border-b border-border/50 text-xs font-mono">
            <div>
              <span className="text-text-muted">TTFB: </span>
              <span className={ttfb != null ? (ttfb < 3000 ? "text-win" : ttfb < 10000 ? "text-gold" : "text-loss") : "text-text-muted"}>
                {ttfb != null ? formatMs(ttfb) : streaming ? "ожидание..." : "—"}
              </span>
            </div>
            <div>
              <span className="text-text-muted">Время: </span>
              <span className="text-text-primary">{formatMs(elapsed)}</span>
            </div>
            <div>
              <span className="text-text-muted">Символов: </span>
              <span className="text-text-primary">{stream.length || (history[0]?.text.length ?? 0)}</span>
            </div>
            {streaming && (
              <Loader2 size={12} className="animate-spin text-accent ml-auto" />
            )}
          </div>
        )}

        {/* Stream output */}
        <div ref={scrollRef} className="flex-1 overflow-y-auto px-5 py-4 min-h-[200px]">
          {error && (
            <div className="p-3 rounded-lg bg-loss/10 border border-loss/20 text-loss text-sm mb-3">
              {error}
            </div>
          )}

          {streaming && stream && (
            <div className="text-sm text-text-primary whitespace-pre-wrap leading-relaxed">
              {stream}
              <span className="inline-block w-0.5 h-4 bg-accent animate-pulse ml-0.5 align-text-bottom" />
            </div>
          )}

          {streaming && !stream && (
            <div className="flex items-center gap-2 text-text-muted text-sm py-8 justify-center">
              <Loader2 size={16} className="animate-spin text-accent" />
              Ожидание первого токена...
            </div>
          )}

          {!streaming && !error && history.length === 0 && (
            <div className="text-text-muted text-sm text-center py-8">
              Отправьте сообщение, чтобы протестировать стриминг AI.
              <br />
              <span className="text-text-muted/60 text-xs">
                Контекст: Jinx (BOTTOM) vs Caitlyn, 10 минута, Gold II
              </span>
            </div>
          )}

          {!streaming && history.length > 0 && (
            <div className="flex flex-col gap-3">
              {history.map((entry, i) => (
                <div key={entry.timestamp} className={`p-3 rounded-lg border text-sm ${i === 0 ? "bg-bg-secondary/50 border-border" : "bg-bg-primary/30 border-border/30 opacity-60"}`}>
                  <div className="whitespace-pre-wrap leading-relaxed text-text-primary">
                    {entry.text}
                  </div>
                  <div className="flex gap-3 mt-2 text-[10px] font-mono text-text-muted">
                    <span>TTFB: {entry.ttfb != null ? formatMs(entry.ttfb) : "—"}</span>
                    <span>Total: {formatMs(entry.totalMs)}</span>
                    <span>{entry.text.length} символов</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Input */}
        <div className="px-5 py-4 border-t border-border flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSend()}
            placeholder="Тестовое сообщение для AI..."
            disabled={streaming}
            className="flex-1 px-4 py-2.5 rounded-xl bg-bg-primary border border-border text-text-primary placeholder:text-text-muted
                       focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent transition-all text-sm disabled:opacity-50"
          />
          {history.length > 0 && (
            <button
              onClick={() => setHistory([])}
              disabled={streaming}
              className="px-3 rounded-xl bg-bg-secondary text-text-muted hover:text-text-primary transition-colors disabled:opacity-30"
              title="Очистить историю"
            >
              <Trash2 size={16} />
            </button>
          )}
          <button
            onClick={handleSend}
            disabled={streaming || !input.trim()}
            className="px-4 py-2.5 rounded-xl bg-accent hover:bg-accent-hover text-white text-sm font-medium transition-colors disabled:opacity-30 flex items-center gap-2"
          >
            {streaming ? (
              <Loader2 size={16} className="animate-spin" />
            ) : (
              <Send size={16} />
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
