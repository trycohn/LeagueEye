import { Trophy, TrendingUp, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

const LEADERBOARD = [
  { rank: 1, name: "Hide on bush", lp: 1842, winrate: "58%", wins: 412, losses: 298 },
  { rank: 2, name: "Canyon", lp: 1756, winrate: "61%", wins: 380, losses: 243 },
  { rank: 3, name: "Chovy", lp: 1690, winrate: "59%", wins: 350, losses: 243 },
  { rank: 4, name: "ShowMaker", lp: 1620, winrate: "56%", wins: 420, losses: 330 },
  { rank: 5, name: "Zeus", lp: 1580, winrate: "55%", wins: 450, losses: 368 },
];

const META_CHAMPS = [
  { name: "Ahri", tier: "S+", winrate: "52.4%", pickrate: "18.2%", banrate: "12.1%" },
  { name: "Jinx", tier: "S+", winrate: "51.8%", pickrate: "24.5%", banrate: "8.4%" },
  { name: "LeeSin", tier: "S", winrate: "49.5%", pickrate: "28.1%", banrate: "15.6%" },
  { name: "Thresh", tier: "S", winrate: "50.2%", pickrate: "19.8%", banrate: "5.2%" },
  { name: "Aatrox", tier: "A", winrate: "48.9%", pickrate: "15.4%", banrate: "22.1%" },
];

export function HomeVariant1({ detectedAccount, onBadgeClick }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-6">
      {/* Account Banner (if connected) */}
      {detectedAccount && (
        <div className="mb-6 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <img
              src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
              alt="Profile"
              className="w-12 h-12 rounded-sm"
            />
            <div>
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Клиент подключен</div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button
            onClick={onBadgeClick}
            className="px-4 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors"
          >
            Мой профиль
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Leaderboard */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Trophy size={16} className="text-[#eab308]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Топ игроков (Challenger)</h2>
          </div>
          <div className="p-0">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">Ранг</th>
                  <th className="px-4 py-2 font-semibold">Призыватель</th>
                  <th className="px-4 py-2 font-semibold text-right">LP</th>
                  <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {LEADERBOARD.map((player) => (
                  <tr key={player.rank} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                    <td className="px-4 py-3 font-bold text-[#94a3b8] w-12">{player.rank}</td>
                    <td className="px-4 py-3 font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">
                      {player.name}
                    </td>
                    <td className="px-4 py-3 text-right font-medium text-[#eab308]">{player.lp}</td>
                    <td className="px-4 py-3 text-right">
                      <div className="font-medium text-[#e2e8f0]">{player.winrate}</div>
                      <div className="text-xs text-[#64748b]">{player.wins}W {player.losses}L</div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Полный рейтинг <ChevronRight size={14} />
            </button>
          </div>
        </div>

        {/* Meta Champions */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <TrendingUp size={16} className="text-[#22c55e]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Мета чемпионов (Патч 14.8)</h2>
          </div>
          <div className="p-0">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">Чемпион</th>
                  <th className="px-4 py-2 font-semibold text-center">Тир</th>
                  <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  <th className="px-4 py-2 font-semibold text-right">Пикрейт</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {META_CHAMPS.map((champ) => (
                  <tr key={champ.name} className="hover:bg-[#252838] transition-colors cursor-pointer">
                    <td className="px-4 py-3 flex items-center gap-3">
                      <img
                        src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ.name}.png`}
                        alt={champ.name}
                        className="w-8 h-8 rounded-sm"
                      />
                      <span className="font-bold text-[#e2e8f0]">{champ.name}</span>
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span className={`inline-block px-2 py-0.5 rounded-sm text-xs font-bold ${
                        champ.tier === 'S+' ? 'bg-[#ef4444]/20 text-[#ef4444]' :
                        champ.tier === 'S' ? 'bg-[#f97316]/20 text-[#f97316]' :
                        'bg-[#3b82f6]/20 text-[#3b82f6]'
                      }`}>
                        {champ.tier}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-right font-medium text-[#e2e8f0]">{champ.winrate}</td>
                    <td className="px-4 py-3 text-right text-[#94a3b8]">{champ.pickrate}</td>
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
