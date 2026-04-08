import { SearchBar } from "../SearchBar";
import { Star, ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

const PRO_PLAYERS = [
  { name: "Hide on bush", tag: "KR1", icon: 6, role: "Mid" },
  { name: "Canyon", tag: "KR1", icon: 5410, role: "Mid" },
  { name: "Oner", tag: "KR1", icon: 5410, role: "Jungle" },
  { name: "Zeus", tag: "KR1", icon: 5410, role: "Top" },
];

export function HomeVariant4({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="max-w-5xl mx-auto pt-16 pb-24">
      {/* Header & Search */}
      <div className="text-center mb-12">
        <h1 className="text-4xl font-extrabold text-text-primary mb-6 tracking-tight">
          Статистика призывателей
        </h1>
        <div className="max-w-2xl mx-auto">
          <SearchBar onSearch={onSearch} loading={loading} />
        </div>
      </div>

      {/* Detected Account Banner (OP.GG style) */}
      {detectedAccount && (
        <div className="max-w-2xl mx-auto mb-12">
          <div className="bg-bg-card border border-border rounded-xl p-4 flex items-center justify-between hover:border-accent/50 transition-colors cursor-pointer" onClick={onBadgeClick}>
            <div className="flex items-center gap-4">
              <div className="relative">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                  alt="Profile"
                  className="w-12 h-12 rounded-lg"
                />
                <span className="absolute -bottom-2 -right-2 bg-bg-secondary border border-border text-[10px] font-bold px-1.5 py-0.5 rounded">
                  {detectedAccount.summonerLevel}
                </span>
              </div>
              <div className="text-left">
                <div className="text-xs text-text-muted font-medium mb-0.5">Мой профиль</div>
                <div className="text-base font-bold text-text-primary">
                  {detectedAccount.gameName} <span className="text-text-muted font-normal">#{detectedAccount.tagLine}</span>
                </div>
              </div>
            </div>
            <ChevronRight className="text-text-muted" />
          </div>
        </div>
      )}

      {/* Pro Players Grid */}
      <div className="max-w-4xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-bold text-text-primary flex items-center gap-2">
            <Star size={18} className="text-gold" /> Популярные игроки
          </h2>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4">
          {PRO_PLAYERS.map((player) => (
            <button
              key={player.name}
              onClick={() => onSearch(player.name, player.tag)}
              className="flex items-center gap-3 p-3 rounded-xl bg-bg-card border border-border hover:bg-bg-hover hover:border-border transition-all text-left group"
            >
              <img
                src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${player.icon}.png`}
                alt={player.name}
                className="w-10 h-10 rounded-lg"
              />
              <div className="overflow-hidden">
                <div className="text-sm font-bold text-text-primary truncate group-hover:text-accent transition-colors">
                  {player.name}
                </div>
                <div className="text-xs text-text-muted truncate">
                  #{player.tag} • {player.role}
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
