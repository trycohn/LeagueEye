import type { LaneGoldComparison } from "../lib/types";
import type { GoldOverlayLayout } from "../lib/goldOverlayLayout";
import { championIconUrl } from "../lib/ddragon";
import { RoleIcon } from "./RoleIcon";

function formatGold(gold: number): string {
  const abs = Math.abs(gold);
  if (abs >= 1000) return `${(gold / 1000).toFixed(1)}k`;
  return gold.toString();
}

function formatGoldFull(gold: number): string {
  if (gold >= 1000) return `${(gold / 1000).toFixed(1)}k`;
  return gold.toString();
}

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
  const barBg = "rgba(239,68,68,0.25)";
  const barAlly = diff >= 0 ? "rgba(34,197,94,0.65)" : "rgba(34,197,94,0.35)";

  const champImg = (name: string) => (
    <img
      src={championIconUrl(name)}
      alt={name}
      className="w-6 h-6 rounded shrink-0"
      onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
    />
  );

  // ── c1: compact + иконки ролей + короткий бар (75% ширины) ──────────────
  if (layout === "c1") {
    return (
      <div className="flex items-center gap-1.5">
        <RoleIcon role={lane.role} size={11} />
        {champImg(lane.allyChampionName)}
        <div className="flex flex-col justify-center" style={{ width: "75%" }}>
          <div className="flex items-center gap-1">
            <div
              className="flex-1 h-1.5 rounded-full overflow-hidden flex"
              style={{ background: barBg }}
            >
              <div
                className="h-full rounded-l-full transition-all duration-700"
                style={{ width: `${allyPct}%`, background: barAlly }}
              />
            </div>
            <span className={`text-[9px] font-bold tabular-nums shrink-0 w-[2.2rem] text-right ${diffColor}`}>
              {diffText}
            </span>
          </div>
        </div>
        {champImg(lane.enemyChampionName)}
      </div>
    );
  }

  // ── c2: compact + иконки ролей + числа вместо бара ──────────────────────
  if (layout === "c2") {
    return (
      <div className="flex items-center gap-1.5">
        <RoleIcon role={lane.role} size={11} />
        {champImg(lane.allyChampionName)}
        <div className="flex-1 flex items-center justify-center gap-1 min-w-0">
          <span className="text-[9px] font-semibold tabular-nums text-win opacity-80">
            {formatGoldFull(lane.allyGold)}
          </span>
          <span className={`text-[10px] font-bold tabular-nums ${diffColor} shrink-0`}>
            {diffText}
          </span>
          <span className="text-[9px] font-semibold tabular-nums text-loss opacity-80">
            {formatGoldFull(lane.enemyGold)}
          </span>
        </div>
        {champImg(lane.enemyChampionName)}
      </div>
    );
  }

  // ── classic ──────────────────────────────────────────────────────────────
  if (layout === "classic") {
    return (
      <div className="flex items-center gap-2">
        <img
          src={championIconUrl(lane.allyChampionName)}
          alt={lane.allyChampionName}
          className="w-8 h-8 rounded shrink-0"
          onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
        />
        <div className="flex-1 min-w-0">
          <p className={`text-[10px] font-bold text-center leading-tight ${diffColor}`}>{diffText}</p>
          <div className="h-2.5 rounded-full overflow-hidden flex" style={{ background: barBg }}>
            <div className="h-full rounded-l-full transition-all duration-700"
              style={{ width: `${allyPct}%`, background: barAlly }} />
          </div>
        </div>
        <img
          src={championIconUrl(lane.enemyChampionName)}
          alt={lane.enemyChampionName}
          className="w-8 h-8 rounded shrink-0"
          onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
        />
      </div>
    );
  }

  // ── compact / single / micro ─────────────────────────────────────────────
  if (layout === "single") {
    return (
      <div className="flex items-center gap-1.5">
        <RoleIcon role={lane.role} size={11} />
        {champImg(lane.allyChampionName)}
        <div className="flex-1 min-w-0 relative h-5 flex items-center">
          <div className="absolute inset-x-0 top-1/2 -translate-y-1/2 h-2 rounded-full overflow-hidden flex"
            style={{ background: barBg }}>
            <div className="h-full rounded-l-full transition-all duration-700"
              style={{ width: `${allyPct}%`, background: barAlly }} />
          </div>
          <p className={`relative z-[1] w-full text-center font-bold leading-none text-[8px] ${diffColor} drop-shadow-[0_1px_2px_rgba(0,0,0,0.85)]`}>
            {diffText}
          </p>
        </div>
        {champImg(lane.enemyChampionName)}
      </div>
    );
  }

  if (layout === "micro") {
    return (
      <div className="flex items-center gap-1">
        <img src={championIconUrl(lane.allyChampionName)} alt={lane.allyChampionName}
          className="w-5 h-5 rounded shrink-0"
          onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
        <div className="flex flex-1 min-w-0 items-center gap-1">
          <div className="flex-1 h-1.5 rounded-full overflow-hidden flex min-w-0" style={{ background: barBg }}>
            <div className="h-full rounded-l-full transition-all duration-700"
              style={{ width: `${allyPct}%`, background: barAlly }} />
          </div>
          <span className={`text-[8px] font-bold tabular-nums shrink-0 w-[2.25rem] text-right ${diffColor}`}>
            {diffText}
          </span>
        </div>
        <img src={championIconUrl(lane.enemyChampionName)} alt={lane.enemyChampionName}
          className="w-5 h-5 rounded shrink-0"
          onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
      </div>
    );
  }

  // compact (default)
  return (
    <div className="flex items-center gap-1.5">
      <RoleIcon role={lane.role} size={11} />
      {champImg(lane.allyChampionName)}
      <div className="flex-1 min-w-0">
        <p className={`text-[9px] font-bold text-center leading-tight ${diffColor}`}>{diffText}</p>
        <div className="h-2 rounded-full overflow-hidden flex" style={{ background: barBg }}>
          <div className="h-full rounded-l-full transition-all duration-700"
            style={{ width: `${allyPct}%`, background: barAlly }} />
        </div>
      </div>
      {champImg(lane.enemyChampionName)}
    </div>
  );
}
