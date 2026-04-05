import { useState } from "react";
import type { ChampionStat } from "../lib/types";
import { championIconUrl, positionIconUrl } from "../lib/ddragon";

interface Props {
  stats: ChampionStat[];
}

const ROLES = [
  { key: "ALL", label: "Все" },
  { key: "TOP", label: "Top" },
  { key: "JUNGLE", label: "Jungle" },
  { key: "MIDDLE", label: "Mid" },
  { key: "BOTTOM", label: "ADC" },
  { key: "UTILITY", label: "Sup" },
] as const;

export function ChampionStats({ stats }: Props) {
  const [role, setRole] = useState("ALL");

  if (stats.length === 0) return null;

  const filtered =
    role === "ALL"
      ? stats
      : stats.filter(
          (s) => s.position.toUpperCase() === role
        );

  return (
    <div className="rounded-xl bg-bg-card border border-border p-4">
      <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wider mb-3">
        Чемпионы
      </h3>

      <div className="flex gap-1 mb-3 flex-wrap">
        {ROLES.map((r) => (
          <button
            key={r.key}
            onClick={() => setRole(r.key)}
            className={`flex items-center gap-1 px-2 py-1 rounded text-xs font-medium transition-colors ${
              role === r.key
                ? "bg-accent text-white"
                : "bg-bg-secondary text-text-muted hover:text-text-primary"
            }`}
          >
            {r.key !== "ALL" && (
              <img
                src={positionIconUrl(r.key)}
                alt={r.label}
                className="w-3.5 h-3.5"
                style={{ filter: role === r.key ? "brightness(10)" : "brightness(0.6)" }}
              />
            )}
            {r.label}
          </button>
        ))}
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-text-muted text-xs uppercase">
              <th className="text-left py-2 px-1"></th>
              <th className="text-left py-2 px-1">Чемпион</th>
              <th className="text-center py-2 px-1">Игр</th>
              <th className="text-center py-2 px-1">WR</th>
              <th className="text-center py-2 px-1">KDA</th>
              <th className="text-center py-2 px-1">CS</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((s) => {
              const kdaVal =
                s.avgDeaths === 0
                  ? 99
                  : (s.avgKills + s.avgAssists) / s.avgDeaths;
              return (
                <tr
                  key={s.championName}
                  className="border-t border-border hover:bg-bg-hover transition-colors"
                >
                  <td className="py-1.5 px-1">
                    {s.position && s.position !== "UNKNOWN" && (
                      <img
                        src={positionIconUrl(s.position)}
                        alt={s.position}
                        className="w-4 h-4 opacity-50"
                      />
                    )}
                  </td>
                  <td className="py-1.5 px-1">
                    <div className="flex items-center gap-2">
                      <img
                        src={championIconUrl(s.championName)}
                        alt={s.championName}
                        className="w-7 h-7 rounded"
                      />
                      <span className="font-medium text-text-primary text-xs">
                        {s.championName}
                      </span>
                    </div>
                  </td>
                  <td className="text-center py-1.5 px-1 text-text-secondary text-xs">
                    {s.games}
                  </td>
                  <td className="text-center py-1.5 px-1">
                    <span
                      className={`text-xs font-semibold ${
                        s.winrate >= 60
                          ? "text-win"
                          : s.winrate >= 50
                            ? "text-text-primary"
                            : "text-loss"
                      }`}
                    >
                      {s.winrate}%
                    </span>
                  </td>
                  <td className="text-center py-1.5 px-1">
                    <span
                      className={`text-xs font-medium ${
                        kdaVal >= 4
                          ? "text-gold"
                          : kdaVal >= 3
                            ? "text-win"
                            : kdaVal >= 2
                              ? "text-text-primary"
                              : "text-loss"
                      }`}
                    >
                      {kdaVal >= 99 ? "P" : kdaVal.toFixed(1)}
                    </span>
                  </td>
                  <td className="text-center py-1.5 px-1 text-text-secondary text-xs">
                    {s.avgCs}
                  </td>
                </tr>
              );
            })}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={6} className="text-center py-4 text-text-muted text-xs">
                  Нет данных для этой роли
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
