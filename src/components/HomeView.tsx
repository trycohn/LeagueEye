import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { BarChart, Users, Activity, Swords, Loader2 } from "lucide-react";
import type { GlobalDashboardData } from "../lib/types";

interface Props {
  onSearch: (gameName: string, tagLine: string) => void;
}

function formatNumber(n: number): string {
  return n.toLocaleString("ru-RU");
}

export function HomeView({ onSearch }: Props) {
  const [data, setData] = useState<GlobalDashboardData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<GlobalDashboardData>("get_global_dashboard")
      .then(setData)
      .catch((e) => console.error("get_global_dashboard error:", e))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center py-24 gap-4">
        <Loader2 size={36} className="animate-spin text-accent" />
        <p className="text-text-muted">Загрузка статистики...</p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="flex flex-col items-center justify-center py-24 gap-4">
        <p className="text-text-muted">Не удалось загрузить данные</p>
      </div>
    );
  }

  const statsCards = [
    { label: "Всего игроков", value: formatNumber(data.stats.totalPlayers), icon: Users, color: "text-[#3b82f6]" },
    { label: "Проанализировано матчей", value: formatNumber(data.stats.analyzedMatches), icon: Activity, color: "text-[#22c55e]" },
    { label: "Сыграно часов", value: formatNumber(data.stats.hoursPlayed), icon: BarChart, color: "text-[#eab308]" },
    { label: "Пента-киллов", value: formatNumber(data.stats.pentakills), icon: Swords, color: "text-[#ef4444]" },
  ];

  return (
    <div className="max-w-7xl mx-auto py-6">
      {/* Top Global Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        {statsCards.map((stat, i) => (
          <div key={i} className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-4 flex items-center gap-4">
            <div className={`w-10 h-10 rounded-sm bg-[#1e2130] border border-[#2a2d3a] flex items-center justify-center ${stat.color}`}>
              <stat.icon size={20} />
            </div>
            <div>
              <div className="text-xs font-bold text-[#64748b] uppercase tracking-wider">{stat.label}</div>
              <div className="text-xl font-black text-[#e2e8f0]">{stat.value}</div>
            </div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Best Players by Role */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Users size={16} className="text-[#3b82f6]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Лучшие игроки по ролям</h2>
          </div>
          {data.bestByRole.length > 0 ? (
            <div className="p-0">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                  <tr>
                    <th className="px-4 py-2 font-semibold">Роль</th>
                    <th className="px-4 py-2 font-semibold">Игрок</th>
                    <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                    <th className="px-4 py-2 font-semibold text-right">KDA</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#2a2d3a]">
                  {data.bestByRole.map((p) => (
                    <tr key={p.role} onClick={() => onSearch(p.player, p.tag)} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                      <td className="px-4 py-3 font-bold text-[#94a3b8] uppercase tracking-wider text-xs">{p.role}</td>
                      <td className="px-4 py-3 flex items-center gap-3">
                        <img src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${p.champ}.png`} alt={p.champ} className="w-8 h-8 rounded-sm" />
                        <span className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">{p.player}</span>
                      </td>
                      <td className="px-4 py-3 text-right font-medium text-[#22c55e]">{p.winrate}</td>
                      <td className="px-4 py-3 text-right font-medium text-[#e2e8f0]">{p.kda}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="p-8 text-center text-[#64748b] text-sm">Недостаточно данных</div>
          )}
        </div>

        {/* Highest Winrate Champions */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <BarChart size={16} className="text-[#22c55e]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Чемпионы с высшим винрейтом</h2>
            </div>
            <span className="text-xs text-[#64748b] font-medium">Мин. 5 игр</span>
          </div>
          {data.topWinrates.length > 0 ? (
            <div className="p-0">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                  <tr>
                    <th className="px-4 py-2 font-semibold">Чемпион</th>
                    <th className="px-4 py-2 font-semibold text-right">Игр</th>
                    <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#2a2d3a]">
                  {data.topWinrates.map((champ) => (
                    <tr key={champ.champ} className="hover:bg-[#252838] transition-colors cursor-pointer">
                      <td className="px-4 py-3 flex items-center gap-3">
                        <img src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ.champ}.png`} alt={champ.champ} className="w-8 h-8 rounded-sm border border-[#2a2d3a]" />
                        <span className="font-bold text-[#e2e8f0]">{champ.champ}</span>
                      </td>
                      <td className="px-4 py-3 text-right font-medium text-[#94a3b8]">{champ.games}</td>
                      <td className="px-4 py-3 text-right font-bold text-[#22c55e]">{champ.winrate}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="p-8 text-center text-[#64748b] text-sm">Недостаточно данных</div>
          )}
        </div>
      </div>
    </div>
  );
}
