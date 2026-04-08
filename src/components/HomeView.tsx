import { BarChart, Users, Activity, Swords } from "lucide-react";
import type { DetectedAccount } from "../lib/types";

interface Props {
  onSearch: (gameName: string, tagLine: string) => void;
  loading: boolean;
}

const GLOBAL_STATS = [
  { label: "Всего игроков", value: "1,245", icon: Users, color: "text-[#3b82f6]" },
  { label: "Проанализировано матчей", value: "45,892", icon: Activity, color: "text-[#22c55e]" },
  { label: "Сыграно часов", value: "24,500", icon: BarChart, color: "text-[#eab308]" },
  { label: "Пента-киллов", value: "342", icon: Swords, color: "text-[#ef4444]" },
];

const BEST_BY_ROLE = [
  { role: "Top", player: "Zeus", champ: "Aatrox", winrate: "65%", kda: "3.2" },
  { role: "Jungle", player: "Canyon", champ: "LeeSin", winrate: "68%", kda: "4.1" },
  { role: "Mid", player: "Chovy", champ: "Ahri", winrate: "70%", kda: "5.5" },
  { role: "ADC", player: "Gumayusi", champ: "Jinx", winrate: "62%", kda: "4.8" },
  { role: "Support", player: "Keria", champ: "Thresh", winrate: "64%", kda: "3.9" },
];

const TOP_WINRATES = [
  { champ: "Janna", winrate: "54.2%", games: 1200 },
  { champ: "KogMaw", winrate: "53.8%", games: 2400 },
  { champ: "Zac", winrate: "53.5%", games: 800 },
  { champ: "Taliyah", winrate: "53.1%", games: 650 },
  { champ: "Shen", winrate: "52.9%", games: 1100 },
];

export function HomeView({ onSearch, loading }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-6">
      {/* Top Global Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        {GLOBAL_STATS.map((stat, i) => (
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
                {BEST_BY_ROLE.map((p) => (
                  <tr key={p.role} onClick={() => onSearch(p.player, "KR1")} className="hover:bg-[#252838] transition-colors cursor-pointer group">
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
        </div>

        {/* Highest Winrate Champions */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <BarChart size={16} className="text-[#22c55e]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Чемпионы с высшим винрейтом</h2>
            </div>
            <span className="text-xs text-[#64748b] font-medium">Мин. 500 игр</span>
          </div>
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
                {TOP_WINRATES.map((champ) => (
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
        </div>
      </div>
    </div>
  );
}
