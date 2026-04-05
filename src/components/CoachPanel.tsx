import { useState } from "react";
import { useAiCoach } from "../hooks/useAiCoach";
import { CoachText } from "./CoachText";
import { Sparkles, Loader2, ChevronDown, Trash2, AlertCircle } from "lucide-react";
import { timeAgo } from "../lib/ddragon";

export function CoachPanel() {
  const { messages, currentStream, isStreaming, error, requestAdvice, clearMessages } =
    useAiCoach();
  const [collapsed, setCollapsed] = useState(false);

  const latestMessage = messages.length > 0 ? messages[messages.length - 1] : null;
  const olderMessages = messages.length > 1 ? messages.slice(0, -1).reverse() : [];

  return (
    <div className="rounded-xl border border-accent/30 bg-bg-secondary/50 overflow-hidden">
      {/* Header */}
      <div
        className="flex items-center justify-between p-4 cursor-pointer hover:bg-bg-hover/50 transition-colors"
        onClick={() => setCollapsed(!collapsed)}
      >
        <div className="flex items-center gap-2">
          <Sparkles size={18} className="text-accent" />
          <h3 className="text-sm font-bold text-accent">AI Тренер</h3>
          {messages.length > 0 && (
            <span className="text-xs text-text-muted bg-bg-primary/50 px-1.5 py-0.5 rounded">
              {messages.length}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={(e) => {
              e.stopPropagation();
              requestAdvice();
            }}
            disabled={isStreaming}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-accent text-white text-xs font-medium hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isStreaming ? (
              <>
                <Loader2 size={12} className="animate-spin" />
                Анализ...
              </>
            ) : (
              <>
                <Sparkles size={12} />
                Получить совет
              </>
            )}
          </button>
          {messages.length > 0 && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                clearMessages();
              }}
              className="p-1.5 rounded-lg text-text-muted hover:text-text-secondary hover:bg-bg-hover transition-colors"
              title="Очистить историю"
            >
              <Trash2 size={14} />
            </button>
          )}
          <ChevronDown
            size={16}
            className={`text-text-muted transition-transform ${collapsed ? "-rotate-90" : ""}`}
          />
        </div>
      </div>

      {/* Content */}
      {!collapsed && (
        <div className="px-4 pb-4 max-h-[70vh] overflow-y-auto">
          {/* Error */}
          {error && (
            <div className="flex items-start gap-2 p-3 rounded-lg bg-loss/10 border border-loss/20 mb-3">
              <AlertCircle size={14} className="text-loss mt-0.5 shrink-0" />
              <p className="text-xs text-loss">{error}</p>
            </div>
          )}

          {/* Streaming */}
          {isStreaming && currentStream && (
            <div className="p-3 rounded-lg bg-bg-primary/50 border border-accent/20">
              <CoachText text={currentStream} />
              <span className="inline-block w-0.5 h-4 bg-accent animate-pulse ml-0.5 align-text-bottom" />
            </div>
          )}

          {/* Streaming but no text yet */}
          {isStreaming && !currentStream && (
            <div className="flex items-center gap-2 p-3 rounded-lg bg-bg-primary/50 border border-accent/20">
              <Loader2 size={14} className="animate-spin text-accent" />
              <span className="text-sm text-text-muted">Анализирую ситуацию...</span>
            </div>
          )}

          {/* Latest message */}
          {!isStreaming && latestMessage && (
            <div className="p-3 rounded-lg bg-bg-primary/50 border border-border">
              <CoachText text={latestMessage.text} />
              <p className="text-xs text-text-muted mt-2">
                {timeAgo(latestMessage.timestamp)}
              </p>
            </div>
          )}

          {/* No messages yet */}
          {!isStreaming && messages.length === 0 && !error && (
            <p className="text-sm text-text-muted text-center py-3">
              Нажмите «Получить совет» для анализа текущей игры
            </p>
          )}

          {/* Older messages */}
          {olderMessages.length > 0 && (
            <div className="mt-3 border-t border-border pt-3 flex flex-col gap-2">
              {olderMessages.map((msg, i) => (
                <div
                  key={i}
                  className="p-2.5 rounded-lg bg-bg-primary/30 border border-border/50"
                >
                  <CoachText text={msg.text} muted />
                  <p className="text-xs text-text-muted mt-1.5">
                    {timeAgo(msg.timestamp)}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

