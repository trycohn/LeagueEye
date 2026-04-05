import type { MasteryInfo } from "../lib/types";
import { championIconUrl, formatPoints } from "../lib/ddragon";
import { useChampionNames } from "../hooks/useChampionNames";

interface Props {
  mastery: MasteryInfo[];
}

export function MasteryList({ mastery }: Props) {
  const championNames = useChampionNames();

  if (mastery.length === 0) return null;

  return (
    <div className="rounded-xl bg-bg-card border border-border p-4">
      <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wider mb-3">
        Champion Mastery
      </h3>
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
        {mastery.map((m) => {
          const name = championNames[m.championId] || `#${m.championId}`;
          return (
            <div
              key={m.championId}
              className="flex flex-col items-center gap-1.5 p-2.5 rounded-lg bg-bg-secondary hover:bg-bg-hover transition-colors"
            >
              <img
                src={championIconUrl(name)}
                alt={name}
                className="w-10 h-10 rounded-lg"
                onError={(e) => {
                  (e.target as HTMLImageElement).style.display = "none";
                }}
              />
              <span className="text-xs font-medium text-text-primary truncate max-w-full">
                {name}
              </span>
              <span className="text-xs text-gold font-medium">
                M{m.championLevel}
              </span>
              <span className="text-[10px] text-text-muted">
                {formatPoints(m.championPoints)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
