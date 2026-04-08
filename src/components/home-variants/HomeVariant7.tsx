import { Trophy, Flame, Crosshair, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

// Mock data aggregated from DB
const DB_LEADERBOARD = [
  { rank: 1, name: "Hide on bush", tag: "KR1", tier: "Challenger", lp: 1842, winrate: "58%" },
  { rank: 2, name: "Chovy", tag: "KR1", tier: "Challenger", lp: 1690, winrate: "61%" },
  { rank: 3, name: "Dantes", tag: "EUW", tier: "Grandmaster", lp: 750, winrate: "54%" },
  { rank: 4, name: "Nemesis", tag: "EUW", tier: "Master", lp: 420, winrate: "56%" },
  { rank: 5, name: "PlayerOne", tag: "RU1", tier: "Diamond 1", lp: 85, winrate: "52%" },
];

const DB_POPULAR_CHAMPS = [
  { champ: "Jinx", picks: 12450, winrate: "51.2%", kda: "3.2" },
  { champ: "LeeSin", picks: 11200, winrate: "49.8%", kda: "2.8" },
  { champ: "Ahri", picks: 9800, winrate: "52.1%", kda: "3.5" },
  { champ: "Thresh", picks: 8900, winrate: "50.5%", kda: "2.9" },
  { champ: "Yasuo", picks: 8500, winrate: "48.2%", kda: "2.1" },
];

const DB_PENTAKILLS = [
  { player: "Gumayusi", champ: "Jinx", matchId: "KR_123456", time: "2 часа назад" },
  { player: "ShowMaker", champ: "Kassadin", matchId: "KR_123457", time: "5 часов назад" },
  { player: "Dantes", champ: "Hecarim", matchId: "EUW_98765", time: "Вчера" },
];

export function HomeVariant7({ detectedAccount, onSearch, onBadgeClick }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-6">
      {detectedAccount && (
        <div className="mb-6 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <img
              src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
              alt="Profile"
              className="w-12 h-12 rounded-sm"
            />
            <div>
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Добро пожаловать</div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button onClick={onBadgeClick} className="px-4 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors">
            Мой профиль
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Leaderboard & Pentakills */}
        <div className="lg:col-span-7 space-y-6">
          {/* Leaderboard */}
          <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
            <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Trophy size={16} className="text-[#eab308]" />
                <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Топ игроков (База данных)</h2>
              </div>
              <span className="text-xs text-[#64748b] font-medium">Обновлено 5 мин назад</span>
            </div>
            <div className="p-0">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                  <tr>
                    <th className="px-4 py-2 font-semibold">#</th>
                    <th className="px-4 py-2 font-semibold">Призыватель</th>
                    <th className="px-4 py-2 font-semibold text-right">Ранг</th>
                    <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#2a2d3a]">
                  {DB_LEADERBOARD.map((p) => (
                    <tr key={p.rank} onClick={() => onSearch(p.name, p.tag)} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                      <td className="px-4 py-3 font-bold text-[#94a3b8] w-10">{p.rank}</td>
                      <td className="px-4 py-3 font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">
                        {p.name} <span className="text-xs text-[#64748b] font-normal">#{p.tag}</span>
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="font-bold text-[#eab308]">{p.tier}</div>
                        <div className="text-xs text-[#94a3b8]">{p.lp} LP</div>
                      </td>
                      <td className="px-4 py-3 text-right font-medium text-[#22c55e]">{p.winrate}</td>
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

          {/* Hall of Fame: Pentakills */}
          <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
            <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
              <Crosshair size={16} className="text-[#ef4444]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Зал славы: Недавние Пента-киллы</h2>
            </div>
            <div className="p-0 divide-y divide-[#2a2d3a]">
              {DB_PENTAKILLS.map((pk, i) => (
                <div key={i} className="px-4 py-3 flex items-center justify-between hover:bg-[#252838] transition-colors cursor-pointer">
                  <div className="flex items-center gap-3">
                    <img src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${pk.champ}.png`} alt={pk.champ} className="w-10 h-10 rounded-sm border border-[#ef4444]" />
                    <div>
                      <div className="text-sm font-bold text-[#e2e8f0]">{pk.player}</div>
                      <div className="text-xs font-black text-[#ef4444] uppercase tracking-wider">Penta Kill!</div>
                    </div>
                  </div>
                  <div className="text-right text-xs text-[#64748b] font-medium">{pk.time}</div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Right Column: Popular Champions */}
        <div className="lg:col-span-5">
          <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col h-full">
            <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
              <Flame size={16} className="text-[#f97316]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Популярные чемпионы</h2>
            </div>
            <div className="p-0 flex-1">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                  <tr>
                    <th className="px-4 py-2 font-semibold">Чемпион</th>
                    <th className="px-4 py-2 font-semibold text-right">Игр</th>
                    <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#2a2d3a]">
                  {DB_POPULAR_CHAMPS.map((champ) => (
                    <tr key={champ.champ} className="hover:bg-[#252838] transition-colors cursor-pointer">
                      <td className="px-4 py-3 flex items-center gap-3">
                        <img src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ.champ}.png`} alt={champ.champ} className="w-8 h-8 rounded-sm" />
                        <span className="font-bold text-[#e2e8f0]">{champ.champ}</span>
                      </td>
                      <td className="px-4 py-3 text-right font-medium text-[#94a3b8]">{(champ.picks / 1000).toFixed(1)}k</td>
                      <td className="px-4 py-3 text-right font-medium text-[#22c55e]">{champ.winrate}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="px-4 py-3 border-t border-[#2a2d3a] text-center mt-auto">
              <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
                Вся мета <ChevronRight size={14} />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
