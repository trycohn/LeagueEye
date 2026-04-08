import { SearchBar } from "../SearchBar";
import { History, Activity, Zap, Play } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

const FREE_CHAMPS = ["Ahri", "LeeSin", "Jinx", "Yasuo", "Thresh"];

export function HomeVariant5({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="max-w-7xl mx-auto py-10 px-4">
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        
        {/* Left Column: Search & Account */}
        <div className="lg:col-span-7 space-y-8">
          {/* Welcome Banner */}
          {detectedAccount ? (
            <div className="bg-bg-card rounded-2xl p-6 border border-border flex flex-col sm:flex-row items-center justify-between gap-6 relative overflow-hidden">
              <div className="absolute right-0 top-0 w-1/2 h-full bg-gradient-to-l from-accent/10 to-transparent pointer-events-none" />
              <div className="flex items-center gap-5 z-10">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                  alt="Profile"
                  className="w-16 h-16 rounded-2xl border-2 border-bg-primary shadow-lg"
                />
                <div>
                  <p className="text-text-muted text-sm font-medium mb-1">С возвращением,</p>
                  <h2 className="text-2xl font-bold text-text-primary">
                    {detectedAccount.gameName} <span className="text-text-muted font-normal">#{detectedAccount.tagLine}</span>
                  </h2>
                </div>
              </div>
              <button
                onClick={onBadgeClick}
                className="z-10 px-6 py-2.5 bg-accent hover:bg-accent-hover text-white font-medium rounded-xl transition-colors whitespace-nowrap"
              >
                Мой профиль
              </button>
            </div>
          ) : (
            <div className="bg-bg-card rounded-2xl p-8 border border-border text-center">
              <Activity className="w-12 h-12 text-accent mx-auto mb-4 opacity-80" />
              <h2 className="text-2xl font-bold text-text-primary mb-2">Клиент не найден</h2>
              <p className="text-text-muted">Запустите League of Legends для автоматической синхронизации.</p>
            </div>
          )}

          {/* Search Section */}
          <div>
            <h3 className="text-lg font-bold text-text-primary mb-4">Поиск призывателя</h3>
            <SearchBar onSearch={onSearch} loading={loading} />
            
            <div className="mt-8">
              <h4 className="text-sm font-bold text-text-muted mb-3 flex items-center gap-2 uppercase tracking-wider">
                <History size={14} /> Недавние
              </h4>
              <div className="flex flex-wrap gap-2">
                {["Dantes#KR1", "Chovy#KR1", "ShowMaker#KR1"].map((player) => (
                  <button
                    key={player}
                    onClick={() => onSearch(player.split("#")[0], player.split("#")[1])}
                    className="px-4 py-2 rounded-lg bg-bg-card border border-border hover:border-accent/50 hover:bg-bg-hover transition-colors text-sm font-medium text-text-primary"
                  >
                    {player.split("#")[0]} <span className="text-text-muted">#{player.split("#")[1]}</span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Right Column: Hub / News */}
        <div className="lg:col-span-5 space-y-6">
          {/* Live Game Promo */}
          <div className="bg-gradient-to-br from-bg-card to-bg-primary rounded-2xl p-6 border border-border relative overflow-hidden group">
            <div className="absolute inset-0 bg-[url('https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Yasuo_0.jpg')] bg-cover bg-center opacity-10 group-hover:opacity-20 transition-opacity duration-500" />
            <div className="relative z-10">
              <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-loss/10 text-loss text-xs font-bold mb-4 border border-loss/20">
                <span className="w-1.5 h-1.5 rounded-full bg-loss animate-pulse" />
                LIVE TRACKER
              </div>
              <h3 className="text-xl font-bold text-white mb-2">Анализ матча в реальном времени</h3>
              <p className="text-text-secondary text-sm mb-6 max-w-[80%]">
                Узнайте ранги, винрейты и мейнов ваших союзников и противников прямо во время загрузки игры.
              </p>
              <button className="flex items-center gap-2 text-sm font-bold text-accent group-hover:text-accent-hover transition-colors">
                <Play size={16} fill="currentColor" /> Как это работает
              </button>
            </div>
          </div>

          {/* Free Rotation (Mock) */}
          <div className="bg-bg-card rounded-2xl p-6 border border-border">
            <h3 className="text-base font-bold text-text-primary mb-4 flex items-center gap-2">
              <Zap size={16} className="text-gold" /> Бесплатные чемпионы
            </h3>
            <div className="flex gap-3 overflow-x-auto pb-2 scrollbar-hide">
              {FREE_CHAMPS.map((champ) => (
                <div key={champ} className="flex-shrink-0 text-center">
                  <img
                    src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/champion/${champ}.png`}
                    alt={champ}
                    className="w-12 h-12 rounded-xl border border-border mb-2 mx-auto shadow-sm"
                  />
                  <span className="text-xs font-medium text-text-muted">{champ}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}
