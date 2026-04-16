import type { LaneGoldComparison } from "../lib/types";
import { championIconUrl, itemIconUrl } from "../lib/ddragon";
import { RoleIcon } from "./RoleIcon";

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
  const counterTitle = lane.counterItem
    ? `Тебе против ${lane.enemyChampionName}: ${lane.counterItem.name} — ${lane.counterItem.counterReason}${lane.counterItem.buildReason ? ` • ${lane.counterItem.buildReason}` : ""}`
    : `Тебе против ${lane.enemyChampionName}: подходящий контр-предмет не определён`;

  const champImg = (name: string) => (
    <img
      src={championIconUrl(name)}
      alt={name}
      className="w-6 h-6 rounded shrink-0"
      onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
    />
  );

  return (
    <div className="flex items-center justify-between gap-1">
      <div className="flex items-center gap-1.5 shrink-0">
        <RoleIcon role={lane.role} size={12} />
        {champImg(lane.allyChampionName)}
      </div>
      <div className="flex flex-col items-stretch shrink-0 mx-1" style={{ width: 64 }}>
        <p className={`text-[9px] font-bold text-center leading-tight tabular-nums ${diffColor}`}>
          {diffText}
        </p>
        <div className="h-1.5 rounded-full overflow-hidden flex" style={{ background: "rgba(239,68,68,0.25)" }}>
          <div
            className="h-full rounded-l-full transition-all duration-700"
            style={{
              width: `${allyPct}%`,
              background: diff >= 0 ? "rgba(34,197,94,0.65)" : "rgba(34,197,94,0.35)",
            }}
          />
        </div>
      </div>
      <div className="flex items-center gap-1 shrink-0">
        {champImg(lane.enemyChampionName)}
        {lane.counterItem ? (
          <img
            src={itemIconUrl(lane.counterItem.itemId)}
            alt={lane.counterItem.name}
            title={counterTitle}
            className="w-5 h-5 rounded border border-border/70 bg-bg-secondary/70 shrink-0"
            onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
          />
        ) : (
          <div
            title={counterTitle}
            className="w-5 h-5 rounded border border-border/70 bg-bg-secondary/40 shrink-0 flex items-center justify-center text-[9px] font-semibold text-text-muted"
          >
            ?
          </div>
        )}
      </div>
    </div>
  );
}
