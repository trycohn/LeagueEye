import { SearchBar } from "../SearchBar";
import { Eye, Search, History, Star, TrendingUp, Gamepad2 } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

export function HomeVariant2({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 min-h-[70vh] py-8">
      {/* Left Column: Search & Quick Actions */}
      <div className="lg:col-span-4 space-y-8">
        <div>
          <h2 className="text-2xl font-bold text-text-primary mb-2 flex items-center gap-2">
            <Search className="text-accent" /> Поиск игрока
          </h2>
          <p className="text-text-muted mb-6">Найдите статистику любого призывателя</p>
          <SearchBar onSearch={onSearch} loading={loading} />
        </div>

        <div className="bg-bg-card rounded-2xl p-6 border border-border">
          <h3 className="font-semibold text-text-primary mb-4 flex items-center gap-2">
            <History size={18} className="text-text-secondary" /> Недавние поиски
          </h3>
          <div className="space-y-3">
            {/* Mock recent searches */}
            {["Faker#KR1", "Chovy#KR1", "ShowMaker#KR1"].map((player) => (
              <button
                key={player}
                onClick={() => onSearch(player.split("#")[0], player.split("#")[1])}
                className="w-full text-left px-4 py-3 rounded-xl hover:bg-bg-hover transition-colors flex justify-between items-center group"
              >
                <span className="text-text-primary font-medium">{player.split("#")[0]}</span>
                <span className="text-text-muted text-sm group-hover:text-accent transition-colors">#{player.split("#")[1]}</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Right Column: Dashboard or Welcome */}
      <div className="lg:col-span-8">
        {detectedAccount ? (
          <div className="space-y-6">
            <div className="bg-gradient-to-br from-accent/20 to-bg-card border border-accent/20 rounded-3xl p-8 relative overflow-hidden">
              <div className="absolute top-0 right-0 w-64 h-64 bg-accent/10 rounded-full blur-3xl" />
              <div className="relative z-10 flex flex-col md:flex-row items-center gap-8">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                  alt="Profile"
                  className="w-32 h-32 rounded-3xl shadow-xl border-4 border-bg-primary"
                />
                <div className="text-center md:text-left flex-1">
                  <div className="inline-block px-3 py-1 rounded-full bg-bg-primary/50 text-accent font-medium text-sm mb-3">
                    Уровень {detectedAccount.summonerLevel}
                  </div>
                  <h1 className="text-4xl font-bold text-text-primary mb-2">
                    {detectedAccount.gameName}
                    <span className="text-text-muted font-normal text-2xl ml-2">
                      #{detectedAccount.tagLine}
                    </span>
                  </h1>
                  <p className="text-text-secondary mb-6">
                    Добро пожаловать обратно! Ваш клиент запущен и готов к работе.
                  </p>
                  <button
                    onClick={onBadgeClick}
                    className="px-8 py-3 rounded-xl bg-accent hover:bg-accent-hover text-white font-semibold transition-all shadow-lg shadow-accent/20"
                  >
                    Перейти в профиль
                  </button>
                </div>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="bg-bg-card rounded-2xl p-6 border border-border">
                <TrendingUp className="w-8 h-8 text-win mb-4" />
                <h4 className="text-text-muted text-sm font-medium mb-1">Статус</h4>
                <p className="text-xl font-bold text-text-primary">В сети</p>
              </div>
              <div className="bg-bg-card rounded-2xl p-6 border border-border">
                <Gamepad2 className="w-8 h-8 text-accent mb-4" />
                <h4 className="text-text-muted text-sm font-medium mb-1">Последняя игра</h4>
                <p className="text-xl font-bold text-text-primary">Победа</p>
              </div>
              <div className="bg-bg-card rounded-2xl p-6 border border-border">
                <Star className="w-8 h-8 text-gold mb-4" />
                <h4 className="text-text-muted text-sm font-medium mb-1">Мастерство</h4>
                <p className="text-xl font-bold text-text-primary">Топ 3</p>
              </div>
            </div>
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center bg-bg-card/30 rounded-3xl border border-border/50 p-12 text-center">
            <div className="w-24 h-24 bg-bg-card rounded-3xl flex items-center justify-center mb-6 shadow-inner">
              <Eye size={40} className="text-text-muted" />
            </div>
            <h2 className="text-2xl font-bold text-text-primary mb-3">
              Клиент не обнаружен
            </h2>
            <p className="text-text-muted max-w-md mx-auto mb-8">
              Запустите League of Legends, чтобы мы могли автоматически определить ваш профиль и показывать статистику в реальном времени.
            </p>
            <div className="flex items-center gap-2 text-sm font-medium text-accent bg-accent/10 px-4 py-2 rounded-full">
              <div className="w-2 h-2 rounded-full bg-accent animate-pulse" />
              Ожидание клиента...
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
