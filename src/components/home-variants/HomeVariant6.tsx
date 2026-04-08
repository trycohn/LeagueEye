import { SearchBar } from "../SearchBar";
import { Search, Eye, ArrowRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

export function HomeVariant6({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="relative min-h-[75vh] flex flex-col items-center justify-center px-4">
      {/* Subtle Dot Pattern Background */}
      <div 
        className="absolute inset-0 pointer-events-none opacity-[0.03] z-0"
        style={{
          backgroundImage: 'radial-gradient(circle at 2px 2px, white 1px, transparent 0)',
          backgroundSize: '32px 32px'
        }}
      />

      <div className="z-10 w-full max-w-2xl flex flex-col items-center">
        {/* Logo / Branding */}
        <div className="flex flex-col items-center mb-10">
          <div className="w-16 h-16 bg-bg-card border border-border rounded-2xl flex items-center justify-center mb-6 shadow-sm">
            <Eye size={32} className="text-text-primary" />
          </div>
          <h1 className="text-5xl font-black text-text-primary tracking-tight mb-3">
            LeagueEye
          </h1>
          <p className="text-text-secondary text-lg font-medium">
            Аналитика. Статистика. Победа.
          </p>
        </div>

        {/* Main Search */}
        <div className="w-full mb-12">
          <SearchBar onSearch={onSearch} loading={loading} />
        </div>

        {/* Minimalist Account / Recent Section */}
        <div className="w-full max-w-lg border-t border-border/50 pt-8">
          {detectedAccount ? (
            <div className="flex items-center justify-between group cursor-pointer" onClick={onBadgeClick}>
              <div className="flex items-center gap-4">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                  alt="Profile"
                  className="w-10 h-10 rounded-full border border-border"
                />
                <div>
                  <div className="text-sm font-bold text-text-primary group-hover:text-accent transition-colors">
                    {detectedAccount.gameName} <span className="text-text-muted font-normal">#{detectedAccount.tagLine}</span>
                  </div>
                  <div className="text-xs text-text-muted">Уровень {detectedAccount.summonerLevel}</div>
                </div>
              </div>
              <div className="flex items-center gap-2 text-sm font-medium text-text-muted group-hover:text-accent transition-colors">
                Перейти <ArrowRight size={16} />
              </div>
            </div>
          ) : (
            <div>
              <div className="text-xs font-bold text-text-muted uppercase tracking-widest mb-4 flex items-center gap-2">
                <Search size={14} /> Недавние поиски
              </div>
              <div className="flex flex-col gap-1">
                {["Hide on bush#KR1", "Jankos#EUW", "Nemesis#EUW"].map((player) => (
                  <button
                    key={player}
                    onClick={() => onSearch(player.split("#")[0], player.split("#")[1])}
                    className="flex justify-between items-center py-3 px-4 -mx-4 rounded-xl hover:bg-bg-card transition-colors group"
                  >
                    <span className="font-medium text-text-primary group-hover:text-accent transition-colors">
                      {player.split("#")[0]}
                    </span>
                    <span className="text-sm text-text-muted">
                      #{player.split("#")[1]}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
