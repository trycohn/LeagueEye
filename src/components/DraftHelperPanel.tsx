import { useDraftHelper } from "../hooks/useDraftHelper";
import { CoachText } from "./CoachText";
import { Swords, Loader2, AlertCircle, Trash2 } from "lucide-react";

export function DraftHelperPanel() {
  const { result, currentStream, isStreaming, error, requestDraftAdvice, clearResult } =
    useDraftHelper();

  return (
    <div className="rounded-xl border border-emerald-500/30 bg-bg-secondary/50 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4">
        <div className="flex items-center gap-2">
          <Swords size={18} className="text-emerald-400" />
          <h3 className="text-sm font-bold text-emerald-400">Draft Helper</h3>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => requestDraftAdvice()}
            disabled={isStreaming}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 text-white text-xs font-medium hover:bg-emerald-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isStreaming ? (
              <>
                <Loader2 size={12} className="animate-spin" />
                Анализ драфта...
              </>
            ) : (
              <>
                <Swords size={12} />
                Помощь с пиком
              </>
            )}
          </button>
          {result && (
            <button
              onClick={clearResult}
              className="p-1.5 rounded-lg text-text-muted hover:text-text-secondary hover:bg-bg-hover transition-colors"
              title="Очистить"
            >
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="px-4 pb-4">
        {/* Error */}
        {error && (
          <div className="flex items-start gap-2 p-3 rounded-lg bg-loss/10 border border-loss/20 mb-3">
            <AlertCircle size={14} className="text-loss mt-0.5 shrink-0" />
            <p className="text-xs text-loss">{error}</p>
          </div>
        )}

        {/* Streaming */}
        {isStreaming && currentStream && (
          <div className="p-3 rounded-lg bg-bg-primary/50 border border-emerald-500/20">
            <CoachText text={currentStream} />
            <span className="inline-block w-0.5 h-4 bg-emerald-400 animate-pulse ml-0.5 align-text-bottom" />
          </div>
        )}

        {/* Streaming but no text yet */}
        {isStreaming && !currentStream && (
          <div className="flex items-center gap-2 p-3 rounded-lg bg-bg-primary/50 border border-emerald-500/20">
            <Loader2 size={14} className="animate-spin text-emerald-400" />
            <span className="text-sm text-text-muted">Анализирую драфт...</span>
          </div>
        )}

        {/* Result */}
        {!isStreaming && result && !error && (
          <div className="p-3 rounded-lg bg-bg-primary/50 border border-emerald-500/20">
            <CoachText text={result} />
          </div>
        )}

        {/* No result yet */}
        {!isStreaming && !result && !error && (
          <p className="text-sm text-text-muted text-center py-3">
            Нажмите &laquo;Помощь с пиком&raquo; для рекомендаций по драфту
          </p>
        )}
      </div>
    </div>
  );
}
