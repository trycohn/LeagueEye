import { PlayCircle, Eye, Clock } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

const LIVE_GAMES = [
  { p1: "Faker", p2: "Chovy", champ1: "Ahri", champ2: "Azir", time: "14:23", elo: "Challenger 1200 LP" },
  { p1: "Oner", p2: "Canyon", champ1: "LeeSin", champ2: "Nidalee", time: "08:45", elo: "Challenger 1450 LP" },
  { p1: "Zeus", p2: "Doran", champ1: "Aatrox", champ2: "Gnar", time: "22:10", elo: "Challenger 1100 LP" },
  { p1: "Gumayusi", p2: "Viper", champ1: "Jinx", champ2: "Zeri", time: "05:30", elo: "Grandmaster 800 LP" },
  { p1: "Keria", p2: "Lehends", champ1: "Thresh", champ2: "Nautilus", time: "31:05", elo: "Challenger 1300 LP" },
  { p1: "ShowMaker", p2: "Bdd", champ1: "Syndra", champ2: "Orianna", time: "18:50", elo: "Challenger 1600 LP" },
];

export function HomeVariant2({ detectedAccount, onBadgeClick }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-6">
      {/* Account Banner (if connected) */}
      {detectedAccount && (
        <div className="mb-6 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-4 flex items-center justify-between shadow-sm">
          <div className="flex items-center gap-4">
            <img
              src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
              alt="Profile"
              className="w-12 h-12 rounded-sm border border-[#2a2d3a]"
            />
            <div>
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider flex items-center gap-1 mb-0.5">
                <div className="w-1.5 h-1.5 rounded-full bg-[#22c55e]"></div>
                В сети
              </div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button
            onClick={onBadgeClick}
            className="px-4 py-2 bg-[#1e2130] border border-[#2a2d3a] hover:bg-[#252838] text-[#e2e8f0] text-sm font-bold rounded-sm transition-colors"
          >
            Перейти в профиль
          </button>
        </div>
      )}

      <div className="mb-4 flex items-center justify-between border-b border-[#2a2d3a] pb-3">
        <h2 className="text-lg font-bold text-[#e2e8f0] uppercase tracking-wider flex items-center gap-2">
          <PlayCircle size={20} className="text-[#ef4444]" />
          Live Spectate (High Elo)
        </h2>
        <div className="text-xs font-bold text-[#94a3b8] uppercase tracking-wider">
          Сейчас играют
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {LIVE_GAMES.map((game, i) => (
          <div key={i} className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm hover:border-[#3b82f6] transition-colors cursor-pointer group">
            <div className="px-4 py-2 border-b border-[#2a2d3a] flex items-center justify-between bg-[#1e2130]">
              <div className="text-xs font-bold text-[#eab308] uppercase tracking-wider flex items-center gap-1.5">
                <Eye size={12} /> {game.elo}
              </div>
              <div className="text-xs font-bold text-[#ef4444] flex items-center gap-1 animate-pulse">
                <Clock size={12} /> {game.time}
              </div>
            </div>
            <div className="p-4 grid grid-cols-3 items-center gap-2">
              {/* Blue Team Player */}
              <div className="text-center flex flex-col items-center gap-2">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${game.champ1}.png`}
                  alt={game.champ1}
                  className="w-12 h-12 rounded-sm border-2 border-[#3b82f6]"
                />
                <div className="text-xs font-bold text-[#e2e8f0] truncate w-full group-hover:text-[#3b82f6] transition-colors">{game.p1}</div>
              </div>
              
              {/* VS */}
              <div className="text-center text-xs font-black text-[#64748b] italic">
                VS
              </div>
              
              {/* Red Team Player */}
              <div className="text-center flex flex-col items-center gap-2">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${game.champ2}.png`}
                  alt={game.champ2}
                  className="w-12 h-12 rounded-sm border-2 border-[#ef4444]"
                />
                <div className="text-xs font-bold text-[#e2e8f0] truncate w-full group-hover:text-[#ef4444] transition-colors">{game.p2}</div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
