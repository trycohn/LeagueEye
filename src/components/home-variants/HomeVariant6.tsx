import { Users, Globe, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

// Mock data from `accounts` and `rank_snapshots` tables
const LOCAL_LEADERBOARD = [
  { rank: 1, name: "Hide on bush", tag: "KR1", tier: "Challenger", lp: 1842, icon: 6 },
  { rank: 2, name: "Chovy", tag: "KR1", tier: "Challenger", lp: 1690, icon: 5410 },
  { rank: 3, name: "Dantes", tag: "EUW", tier: "Grandmaster", lp: 750, icon: 1234 },
  { rank: 4, name: "Nemesis", tag: "EUW", tier: "Master", lp: 420, icon: 4321 },
  { rank: 5, name: "PlayerOne", tag: "RU1", tier: "Diamond 1", lp: 85, icon: 987 },
];

// Mock data from aggregated `match_participants` table
const SERVER_META = [
  { champ: "Jinx", picks: 12450, winrate: "51.2%", kda: "3.2" },
  { champ: "LeeSin", picks: 11200, winrate: "49.8%", kda: "2.8" },
  { champ: "Ahri", picks: 9800, winrate: "52.1%", kda: "3.5" },
  { champ: "Thresh", picks: 8900, winrate: "50.5%", kda: "2.9" },
  { champ: "Yasuo", picks: 8500, winrate: "48.2%", kda: "2.1" },
];

export function HomeVariant6({ detectedAccount, onSearch, onBadgeClick }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-6">
      {/* Account Banner */}
      {detectedAccount && (
        <div className="mb-6 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <img
              src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
              alt="Profile"
              className="w-12 h-12 rounded-sm"
            />
            <div>
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Ваш профиль</div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button
            onClick={onBadgeClick}
            className="px-4 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors"
          >
            Подробная статистика
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Local Leaderboard (from accounts & rank_snapshots) */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Users size={16} className="text-[#3b82f6]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Рейтинг сервера (LeagueEye)</h2>
          </div>
          <div className="p-0 flex-1">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">Ранг</th>
                  <th className="px-4 py-2 font-semibold">Игрок</th>
                  <th className="px-4 py-2 font-semibold text-right">Тир</th>
                  <th className="px-4 py-2 font-semibold text-right">LP</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {LOCAL_LEADERBOARD.map((player) => (
                  <tr 
                    key={player.rank} 
                    onClick={() => onSearch(player.name, player.tag)}
                    className="hover:bg-[#252838] transition-colors cursor-pointer group"
                  >
                    <td className="px-4 py-3 font-bold text-[#94a3b8] w-12">{player.rank}</td>
                    <td className="px-4 py-3 flex items-center gap-3">
                      <img
                        src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${player.icon}.png`}
                        alt={player.name}
                        className="w-8 h-8 rounded-sm border border-[#2a2d3a]"
                      />
                      <div>
                        <span className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">{player.name}</span>
                        <span className="text-xs text-[#64748b] ml-1">#{player.tag}</span>
                      </div>
                    </td>
                    <td className="px-4 py-3 text-right font-bold text-[#eab308]">{player.tier}</td>
                    <td className="px-4 py-3 text-right font-medium text-[#e2e8f0]">{player.lp}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Весь рейтинг <ChevronRight size={14} />
            </button>
          </div>
        </div>

        {/* Server Meta (from match_participants) */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Globe size={16} className="text-[#22c55e]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Популярные чемпионы (База данных)</h2>
          </div>
          <div className="p-0 flex-1">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">Чемпион</th>
                  <th className="px-4 py-2 font-semibold text-right">Игр</th>
                  <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  <th className="px-4 py-2 font-semibold text-right">KDA</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {SERVER_META.map((champ) => (
                  <tr key={champ.champ} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                    <td className="px-4 py-3 flex items-center gap-3">
                      <img
                        src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ.champ}.png`}
                        alt={champ.champ}
                        className="w-8 h-8 rounded-sm"
                      />
                      <span className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">{champ.champ}</span>
                    </td>
                    <td className="px-4 py-3 text-right font-medium text-[#e2e8f0]">{(champ.picks / 1000).toFixed(1)}k</td>
                    <td className="px-4 py-3 text-right font-medium text-[#22c55e]">{champ.winrate}</td>
                    <td className="px-4 py-3 text-right text-[#94a3b8]">{champ.kda}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Вся статистика <ChevronRight size={14} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
