import { Star, History, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

// Mock data from `champion_mastery` and `matches` tables
const TOP_CHAMPIONS = [
  { id: "Jinx", level: 7, points: 1250000, winrate: "62%", kda: "3.8" },
  { id: "Thresh", level: 7, points: 840000, winrate: "55%", kda: "4.2" },
  { id: "Ahri", level: 6, points: 420000, winrate: "58%", kda: "3.1" },
];

const RECENT_MATCHES = [
  { id: "1", win: true, champ: "Jinx", kda: "12/2/8", time: "2 часа назад", type: "Ранговая (Соло)" },
  { id: "2", win: false, champ: "Thresh", kda: "1/5/14", time: "5 часов назад", type: "Ранговая (Соло)" },
  { id: "3", win: true, champ: "Ahri", kda: "8/1/6", time: "Вчера", type: "Ранговая (Соло)" },
  { id: "4", win: true, champ: "Jinx", kda: "15/4/10", time: "Вчера", type: "Ранговая (Соло)" },
  { id: "5", win: false, champ: "Ezreal", kda: "4/6/5", time: "2 дня назад", type: "Обычная" },
];

export function HomeVariant5({ detectedAccount, onBadgeClick }: Props) {
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
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Синхронизация завершена</div>
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
        {/* Top Champions (from champion_mastery) */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Star size={16} className="text-[#eab308]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Мастерство чемпионов</h2>
          </div>
          <div className="p-0 flex-1">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">Чемпион</th>
                  <th className="px-4 py-2 font-semibold text-right">Очки</th>
                  <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  <th className="px-4 py-2 font-semibold text-right">KDA</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {TOP_CHAMPIONS.map((champ) => (
                  <tr key={champ.id} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                    <td className="px-4 py-3 flex items-center gap-3">
                      <div className="relative">
                        <img
                          src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ.id}.png`}
                          alt={champ.id}
                          className="w-10 h-10 rounded-sm border border-[#2a2d3a]"
                        />
                        <div className="absolute -bottom-1.5 -right-1.5 bg-[#1e2130] border border-[#2a2d3a] text-[10px] font-black px-1 rounded-sm text-[#eab308]">
                          {champ.level}
                        </div>
                      </div>
                      <span className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">{champ.id}</span>
                    </td>
                    <td className="px-4 py-3 text-right font-medium text-[#e2e8f0]">{(champ.points / 1000000).toFixed(1)}M</td>
                    <td className="px-4 py-3 text-right font-medium text-[#22c55e]">{champ.winrate}</td>
                    <td className="px-4 py-3 text-right text-[#94a3b8]">{champ.kda}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Все чемпионы <ChevronRight size={14} />
            </button>
          </div>
        </div>

        {/* Recent Matches Overview (from matches) */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <History size={16} className="text-[#3b82f6]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Последние матчи</h2>
          </div>
          <div className="p-0 flex-1">
            <div className="divide-y divide-[#2a2d3a]">
              {RECENT_MATCHES.map((match) => (
                <div key={match.id} className="flex items-center justify-between px-4 py-3 hover:bg-[#252838] transition-colors cursor-pointer border-l-4" style={{ borderLeftColor: match.win ? '#22c55e' : '#ef4444' }}>
                  <div className="flex items-center gap-3">
                    <img
                      src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${match.champ}.png`}
                      alt={match.champ}
                      className="w-10 h-10 rounded-sm"
                    />
                    <div>
                      <div className={`text-sm font-black ${match.win ? 'text-[#22c55e]' : 'text-[#ef4444]'}`}>
                        {match.win ? 'Победа' : 'Поражение'}
                      </div>
                      <div className="text-xs font-bold text-[#64748b]">{match.type}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-sm font-black text-[#e2e8f0]">{match.kda}</div>
                    <div className="text-xs font-medium text-[#64748b]">{match.time}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Полная история <ChevronRight size={14} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
