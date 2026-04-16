import { useEffect } from "react";
import { Loader2, ChevronLeft, RefreshCw } from "lucide-react";
import type { MatchDetail, MatchSummary } from "../lib/types";
import { usePostGameReview } from "../hooks/usePostGameReview";
import { MatchDetailView } from "./MatchDetailView";
import { CoachText } from "./CoachText";
import {
  championIconUrl,
  positionIconUrl,
  formatDuration,
  timeAgo,
} from "../lib/ddragon";

const POSITION_LABEL: Record<string, string> = {
  TOP: "Top",
  JUNGLE: "Jungle",
  MIDDLE: "Mid",
  BOTTOM: "ADC",
  UTILITY: "Support",
};

interface Props {
  matchId: string;
  puuid: string;
  matchSummary: MatchSummary | null;
  matchDetail: MatchDetail | null;
  onBack: () => void;
  onPlayerClick: (gameName: string, tagLine: string) => void;
}

export function PostGameReviewView({
  matchId,
  puuid,
  matchSummary,
  matchDetail,
  onBack,
  onPlayerClick,
}: Props) {
  const {
    reviewText,
    currentStream,
    isStreaming,
    error,
    reviewMatchId,
    requestReview,
    clearReview,
  } = usePostGameReview();

  // Auto-request review when entering view
  useEffect(() => {
    if (reviewMatchId !== matchId) {
      clearReview();
      requestReview(matchId, puuid);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [matchId, puuid]);

  const displayText = reviewText || currentStream;
  const m = matchSummary;

  const kda =
    m && m.deaths === 0
      ? "Perfect"
      : m
        ? ((m.kills + m.assists) / m.deaths).toFixed(1)
        : "0";

  const csPerMin =
    m && m.gameDuration > 0
      ? ((m.cs / m.gameDuration) * 60).toFixed(1)
      : "0";

  return (
    <div className="flex flex-col gap-5">
      <button
        onClick={() => {
          clearReview();
          onBack();
        }}
        className="flex items-center gap-1 text-text-muted hover:text-text-primary text-sm transition-colors w-fit"
      >
        <ChevronLeft size={16} />
        Назад к профилю
      </button>

      {/* Match header */}
      {m && (
        <div
          className="rounded-xl border p-4"
          style={{
            backgroundColor: m.win
              ? "rgba(34,197,94,0.05)"
              : "rgba(239,68,68,0.05)",
            borderColor: m.win
              ? "rgba(34,197,94,0.2)"
              : "rgba(239,68,68,0.2)",
          }}
        >
          <div className="flex items-center gap-4">
            <img
              src={championIconUrl(m.championName)}
              alt={m.championName}
              className="w-14 h-14 rounded-lg"
            />
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <span className="font-bold text-lg text-text-primary">
                  {m.championName}
                </span>
                {m.position &&
                  POSITION_LABEL[m.position.toUpperCase()] && (
                    <img
                      src={positionIconUrl(m.position.toLowerCase())}
                      alt={POSITION_LABEL[m.position.toUpperCase()]}
                      className="w-5 h-5 opacity-70 brightness-200"
                    />
                  )}
                <span
                  className={`text-sm font-semibold ${m.win ? "text-win" : "text-loss"}`}
                >
                  {m.win ? "ПОБЕДА" : "ПОРАЖЕНИЕ"}
                </span>
              </div>
              <div className="flex items-center gap-3 text-sm text-text-secondary mt-1">
                <span className="font-bold">
                  {m.kills}/{m.deaths}/{m.assists}
                </span>
                <span className="text-text-muted">KDA {kda}</span>
                <span className="text-text-muted">
                  {m.cs} CS ({csPerMin}/m)
                </span>
                <span className="text-text-muted">
                  {formatDuration(m.gameDuration)}
                </span>
                <span className="text-text-muted">
                  {timeAgo(m.gameCreation)}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Match detail table */}
      {matchDetail && (
        <div className="rounded-xl bg-bg-card border border-border overflow-hidden">
          <MatchDetailView
            detail={matchDetail}
            onPlayerClick={onPlayerClick}
          />
        </div>
      )}

      {/* AI Review panel */}
      <div className="rounded-xl bg-bg-card border border-border p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wider">
            AI Разбор матча
          </h3>
          {(reviewText || error) && !isStreaming && (
            <button
              onClick={() => requestReview(matchId, puuid, true)}
              className="flex items-center gap-1.5 text-xs text-text-muted hover:text-accent transition-colors"
              title="Перегенерировать разбор"
            >
              <RefreshCw size={12} />
              Обновить
            </button>
          )}
        </div>

        {isStreaming && !displayText && (
          <div className="flex items-center gap-2 text-text-muted text-sm py-4">
            <Loader2 size={16} className="animate-spin" />
            Анализируем матч...
          </div>
        )}

        {displayText && (
          <div className="relative">
            <CoachText text={displayText} />
            {isStreaming && (
              <span className="inline-block w-1.5 h-4 bg-accent animate-pulse ml-0.5 align-middle" />
            )}
          </div>
        )}

        {error && (
          <div className="text-sm text-loss py-2">{error}</div>
        )}
      </div>
    </div>
  );
}
