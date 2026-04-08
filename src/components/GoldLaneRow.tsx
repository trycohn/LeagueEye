import type { LaneGoldComparison } from "../lib/types";
import type { GoldOverlayLayout } from "../lib/goldOverlayLayout";
import { championIconUrl } from "../lib/ddragon";

function formatGold(gold: number): string {
  const abs = Math.abs(gold);
  if (abs >= 1000) return `${(gold / 1000).toFixed(1)}k`;
  return gold.toString();
}

const iconClass: Record<GoldOverlayLayout, string> = {
  classic: "w-8 h-8",
  compact: "w-6 h-6",
  single: "w-6 h-6",
  micro: "w-5 h-5",
};

const rowGap: Record<GoldOverlayLayout, string> = {
  classic: "gap-2",
  compact: "gap-1.5",
  single: "gap-1.5",
  micro: "gap-1",
};

const diffSize: Record<GoldOverlayLayout, string> = {
  classic: "text-[10px]",
  compact: "text-[9px]",
  single: "text-[8px]",
  micro: "text-[8px]",
};

export function GoldLaneRow({
  lane,
  layout,
}: {
  lane: LaneGoldComparison;
  layout: GoldOverlayLayout;
}) {
  const total = lane.allyGold + lane.enemyGold;
  const allyPct = total > 0 ? (lane.allyGold / total) * 100 : 50;
  const diff = lane.goldDiff;
  const diffColor = diff > 0 ? "text-win" : diff < 0 ? "text-loss" : "text-text-muted";
  const diffText = diff > 0 ? `+${formatGold(diff)}` : diff < 0 ? formatGold(diff) : "—";
  const ic = iconClass[layout];
  const barBg = "rgba(239,68,68,0.3)";
  const barAlly =
    diff >= 0 ? "rgba(34,197,94,0.7)" : "rgba(34,197,94,0.4)";

  const img = (name: string, side: "ally" | "enemy") => (
    <img
      src={championIconUrl(name)}
      alt={name}
      className={`${ic} rounded shrink-0 ${side === "enemy" ? "opacity-95" : ""}`}
      onError={(e) => {
        (e.target as HTMLImageElement).style.display = "none";
      }}
    />
  );

  if (layout === "micro") {
    return (
      <div className={`flex items-center ${rowGap.micro}`}>
        {img(lane.allyChampionName, "ally")}
        <div className="flex flex-1 min-w-0 items-center gap-1">
          <div
            className="flex-1 h-1.5 rounded-full overflow-hidden flex min-w-0"
            style={{ background: barBg }}
          >
            <div
              className="h-full rounded-l-full transition-all duration-700"
              style={{ width: `${allyPct}%`, background: barAlly }}
            />
          </div>
          <span className={`${diffSize.micro} font-bold tabular-nums shrink-0 w-[2.25rem] text-right ${diffColor}`}>
            {diffText}
          </span>
        </div>
        {img(lane.enemyChampionName, "enemy")}
      </div>
    );
  }

  if (layout === "single") {
    return (
      <div className={`flex items-center ${rowGap.single}`}>
        {img(lane.allyChampionName, "ally")}
        <div className="flex-1 min-w-0 relative h-5 flex items-center">
          <div
            className="absolute inset-x-0 top-1/2 -translate-y-1/2 h-2 rounded-full overflow-hidden flex"
            style={{ background: barBg }}
          >
            <div
              className="h-full rounded-l-full transition-all duration-700"
              style={{ width: `${allyPct}%`, background: barAlly }}
            />
          </div>
          <p
            className={`relative z-[1] w-full text-center font-bold leading-none ${diffSize.single} ${diffColor} drop-shadow-[0_1px_2px_rgba(0,0,0,0.85)]`}
          >
            {diffText}
          </p>
        </div>
        {img(lane.enemyChampionName, "enemy")}
      </div>
    );
  }

  const barH = layout === "classic" ? "h-2.5" : "h-2";

  return (
    <div className={`flex items-center ${rowGap[layout]}`}>
      {img(lane.allyChampionName, "ally")}
      <div className="flex-1 min-w-0">
        <p className={`${diffSize[layout]} font-bold text-center ${diffColor} leading-tight`}>
          {diffText}
        </p>
        <div
          className={`${barH} rounded-full overflow-hidden flex`}
          style={{ background: barBg }}
        >
          <div
            className="h-full rounded-l-full transition-all duration-700"
            style={{ width: `${allyPct}%`, background: barAlly }}
          />
        </div>
      </div>
      {img(lane.enemyChampionName, "enemy")}
    </div>
  );
}
