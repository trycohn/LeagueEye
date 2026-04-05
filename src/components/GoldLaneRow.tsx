import type { LaneGoldComparison } from "../lib/types";
import { championIconUrl } from "../lib/ddragon";

const ROLE_LABELS: Record<string, string> = {
  TOP: "TOP",
  JUNGLE: "JG",
  MIDDLE: "MID",
  BOTTOM: "ADC",
  UTILITY: "SUP",
};

function formatGold(gold: number): string {
  const abs = Math.abs(gold);
  if (abs >= 1000) return `${(gold / 1000).toFixed(1)}k`;
  return gold.toString();
}

export function GoldLaneRow({ lane }: { lane: LaneGoldComparison }) {
  const total = lane.allyGold + lane.enemyGold;
  const allyPct = total > 0 ? (lane.allyGold / total) * 100 : 50;
  const diff = lane.goldDiff;
  const diffColor = diff > 0 ? "text-win" : diff < 0 ? "text-loss" : "text-text-muted";
  const diffText = diff > 0 ? `+${formatGold(diff)}` : diff < 0 ? formatGold(diff) : "—";

  return (
    <div className="flex items-center gap-2">
      {/* Role label */}
      <span className="text-[10px] text-text-muted w-6 text-center shrink-0">
        {ROLE_LABELS[lane.role] ?? lane.role}
      </span>

      {/* Ally icon */}
      <img
        src={championIconUrl(lane.allyChampionName)}
        alt={lane.allyChampionName}
        className="w-7 h-7 rounded shrink-0"
        onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
      />

      {/* Bar + diff */}
      <div className="flex-1 min-w-0">
        <p className={`text-[10px] font-bold text-center ${diffColor} leading-tight`}>
          {diffText}
        </p>
        <div className="h-2 rounded-full overflow-hidden flex" style={{ background: "rgba(239,68,68,0.3)" }}>
          <div
            className="h-full rounded-l-full transition-all duration-700"
            style={{
              width: `${allyPct}%`,
              background: diff >= 0 ? "rgba(34,197,94,0.7)" : "rgba(34,197,94,0.4)",
            }}
          />
        </div>
      </div>

      {/* Enemy icon */}
      <img
        src={championIconUrl(lane.enemyChampionName)}
        alt={lane.enemyChampionName}
        className="w-7 h-7 rounded shrink-0"
        onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
      />
    </div>
  );
}
