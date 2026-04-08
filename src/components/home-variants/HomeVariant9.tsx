import { Medal, Clock, ShieldAlert, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

const MASTERY_LEADERS = [
  { champ: "Yasuo", player: "Dzukill", points: "4.2M", rank: 1 },
  { champ: "LeeSin", player: "Broxah", points: "3.8M", rank: 2 },
  { champ: "Thresh", player: "MadLife", points: "3.5M", rank: 3 },
  { champ: "Riven", player: "AloisNL", points: "3.1M", rank: 4 },
  { champ: "Shaco", player: "Dantes", points: "2.9M", rank: 5 },
];

const RECENT_PLAYERS = [
  { name: "Faker", tag: "KR1", time: "2 мин назад" },
  { name: "Ruler", tag: "KR1", time: "5 мин назад" },
  { name: "Jankos", tag: "EUW", time: "12 мин назад" },
  { name: "Bwipo", tag: "EUW", time: "18 мин назад" },
  { name: "Sneaky", tag: "NA1", time: "25 мин назад" },
];

const CRAZY_MATCHES = [
  { title: "Самая кровавая игра", desc: "142 убийства за 45 минут", players: ["Hide on bush", "Chovy"], time: "Вчера" },
  { title: "Самая долгая игра", desc: "68 минут напряженной борьбы", players: ["Nemesis", "Dantes"], time: "2 дня назад" },
];

export function HomeVariant9({ detectedAccount, onSearch, onBadgeClick }: Props) {
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
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Аккаунт подключен</div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button onClick={onBadgeClick} className="px-4 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors">
            Моя статистика
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Mastery Leaders */}
        <div className="lg:col-span-2 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Medal size={16} className="text-[#eab308]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Лидеры по мастерству (Сервер)</h2>
          </div>
          <div className="p-0">
            <table className="w-full text-left text-sm">
              <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                <tr>
                  <th className="px-4 py-2 font-semibold">#</th>
                  <th className="px-4 py-2 font-semibold">Чемпион</th>
                  <th className="px-4 py-2 font-semibold">Игрок</th>
                  <th className="px-4 py-2 font-semibold text-right">Очки</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[#2a2d3a]">
                {MASTERY_LEADERS.map((m) => (
                  <tr key={m.rank} onClick={() => onSearch(m.player, "EUW")} className="hover:bg-[#252838] transition-colors cursor-pointer group">
                    <td className="px-4 py-3 font-bold text-[#94a3b8] w-10">{m.rank}</td>
                    <td className="px-4 py-3 flex items-center gap-3">
                      <img src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${m.champ}.png`} alt={m.champ} className="w-8 h-8 rounded-sm" />
                      <span className="font-bold text-[#e2e8f0]">{m.champ}</span>
                    </td>
                    <td className="px-4 py-3 font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">{m.player}</td>
                    <td className="px-4 py-3 text-right font-black text-[#eab308]">{m.points}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-4 py-3 border-t border-[#2a2d3a] text-center mt-auto">
            <button className="text-xs font-bold text-[#3b82f6] hover:text-[#2563eb] uppercase tracking-wider flex items-center justify-center gap-1 w-full">
              Полный список <ChevronRight size={14} />
            </button>
          </div>
        </div>

        {/* Right Column: Recent Players & Crazy Matches */}
        <div className="space-y-6">
          <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
            <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
              <Clock size={16} className="text-[#3b82f6]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Недавно искали</h2>
            </div>
            <div className="p-0 divide-y divide-[#2a2d3a]">
              {RECENT_PLAYERS.map((p, i) => (
                <div key={i} onClick={() => onSearch(p.name, p.tag)} className="px-4 py-3 flex items-center justify-between hover:bg-[#252838] transition-colors cursor-pointer group">
                  <div className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">
                    {p.name} <span className="text-xs text-[#64748b] font-normal">#{p.tag}</span>
                  </div>
                  <div className="text-xs text-[#64748b] font-medium">{p.time}</div>
                </div>
              ))}
            </div>
          </div>

          <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
            <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
              <ShieldAlert size={16} className="text-[#ef4444]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Рекорды сервера</h2>
            </div>
            <div className="p-0 divide-y divide-[#2a2d3a]">
              {CRAZY_MATCHES.map((m, i) => (
                <div key={i} className="px-4 py-3 hover:bg-[#252838] transition-colors cursor-pointer">
                  <div className="text-xs font-black text-[#ef4444] uppercase tracking-wider mb-1">{m.title}</div>
                  <div className="text-sm font-bold text-[#e2e8f0] mb-1">{m.desc}</div>
                  <div className="flex items-center justify-between text-xs">
                    <div className="text-[#94a3b8]">{m.players.join(" vs ")}</div>
                    <div className="text-[#64748b]">{m.time}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
