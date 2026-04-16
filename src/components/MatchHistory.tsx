import { useEffect, useRef } from "react";
import { Loader2 } from "lucide-react";
import type { MatchSummary } from "../lib/types";
import { MatchCard } from "./MatchCard";

interface Props {
  matches: MatchSummary[];
  hasMore: boolean;
  loadingMore: boolean;
  totalCached: number;
  totalWins: number;
  totalLosses: number;
  onLoadMore: () => void;
  onPlayerClick: (gameName: string, tagLine: string) => void;
  onReview?: (matchId: string) => void;
}

export function MatchHistory({
  matches,
  hasMore,
  loadingMore,
  totalCached,
  totalWins,
  totalLosses,
  onLoadMore,
  onPlayerClick,
  onReview,
}: Props) {
  const sentinelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!hasMore || loadingMore) return;
    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          onLoadMore();
        }
      },
      { rootMargin: "200px" }
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasMore, loadingMore, onLoadMore]);

  if (matches.length === 0) return null;

  const total = totalWins + totalLosses;
  const wr = total > 0 ? ((totalWins / total) * 100).toFixed(0) : "0";

  return (
    <div className="rounded-xl bg-bg-card border border-border p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wider">
          История матчей
        </h3>
        <div className="flex items-center gap-2 text-xs">
          <span className="text-text-muted">{totalCached} игр:</span>
          <span className="text-win">{totalWins}W</span>
          <span className="text-text-muted">/</span>
          <span className="text-loss">{totalLosses}L</span>
          <span className="text-text-muted">·</span>
          <span className={Number(wr) >= 50 ? "text-win" : "text-loss"}>
            {wr}%
          </span>
        </div>
      </div>

      <div className="flex flex-col gap-1.5">
        {matches.map((m) => (
          <MatchCard key={m.matchId} match={m} onPlayerClick={onPlayerClick} onReview={onReview} />
        ))}
      </div>

      {hasMore && <div ref={sentinelRef} className="h-4" />}

      {loadingMore && (
        <div className="flex items-center justify-center gap-2 py-3 text-text-muted text-sm">
          <Loader2 size={14} className="animate-spin" />
          Загрузка...
        </div>
      )}
    </div>
  );
}
