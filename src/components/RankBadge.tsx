import type { RankInfo } from "../lib/types";
import { tierColor, tierDisplayName, queueName, rankEmblemUrl } from "../lib/ddragon";

interface Props {
  rank: RankInfo;
}

export function RankBadge({ rank }: Props) {
  const color = tierColor(rank.tier);
  const total = rank.wins + rank.losses;

  return (
    <div className="inline-flex items-center gap-3 px-4 py-3 rounded-lg bg-bg-secondary border border-border">
      <img
        src={rankEmblemUrl(rank.tier)}
        alt={rank.tier}
        className="w-12 h-12 shrink-0 object-contain"
        style={{ transform: "scale(6)" }}
        onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
      />

      <div className="flex flex-col gap-0.5 min-w-0">
        <span className="text-[11px] text-text-muted uppercase tracking-wider">
          {queueName(rank.queueType)}
        </span>
        <div className="flex items-baseline gap-2">
          <span className="text-base font-bold" style={{ color }}>
            {tierDisplayName(rank.tier)} {rank.rank}
          </span>
          <span className="text-sm text-text-secondary">· {rank.lp} LP</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <span className="text-win">{rank.wins}поб</span>
          <span className="text-loss">{rank.losses}пор</span>
          <span className="text-text-muted">·</span>
          <span className={rank.winrate >= 50 ? "text-win" : "text-loss"}>
            {rank.winrate}%
          </span>
        </div>
        <div className="w-full h-1 bg-bg-primary rounded-full mt-0.5 overflow-hidden max-w-[200px]">
          <div
            className="h-full rounded-full transition-all"
            style={{
              width: `${total > 0 ? (rank.wins / total) * 100 : 0}%`,
              backgroundColor: color,
            }}
          />
        </div>
      </div>
    </div>
  );
}
