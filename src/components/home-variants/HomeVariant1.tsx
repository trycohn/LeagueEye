import { SearchBar } from "../SearchBar";
import { Eye, Trophy, Activity, ArrowRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

export function HomeVariant1({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="flex flex-col items-center justify-center min-h-[70vh] gap-8 relative">
      {/* Background glow */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-accent/10 rounded-full blur-[120px] -z-10 pointer-events-none" />

      <div className="text-center space-y-4 mb-4">
        <div className="flex items-center justify-center gap-3 mb-6">
          <div className="p-4 bg-accent/10 rounded-2xl">
            <Eye size={48} className="text-accent" />
          </div>
        </div>
        <h1 className="text-5xl font-bold text-text-primary tracking-tight">
          League<span className="text-accent">Eye</span>
        </h1>
        <p className="text-text-secondary text-lg max-w-md mx-auto">
          Ваш персональный помощник в League of Legends. Статистика, история матчей и AI-аналитика.
        </p>
      </div>

      <div className="w-full max-w-2xl bg-bg-card/50 backdrop-blur-sm p-6 rounded-3xl border border-border/50 shadow-2xl">
        <SearchBar onSearch={onSearch} loading={loading} />
      </div>

      {detectedAccount && (
        <div className="mt-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
          <button
            onClick={onBadgeClick}
            className="group flex items-center gap-4 p-4 pr-6 rounded-2xl bg-bg-card border border-border hover:border-accent/50 transition-all hover:bg-bg-hover"
          >
            <div className="relative">
              <img
                src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                alt="Profile"
                className="w-14 h-14 rounded-xl"
              />
              <div className="absolute -bottom-2 -right-2 bg-bg-primary px-2 py-0.5 rounded-md text-xs font-bold border border-border">
                {detectedAccount.summonerLevel}
              </div>
            </div>
            <div className="text-left">
              <p className="text-sm text-text-muted mb-0.5">Добро пожаловать</p>
              <p className="font-bold text-text-primary text-lg">
                {detectedAccount.gameName}
                <span className="text-text-muted font-normal text-sm ml-1">
                  #{detectedAccount.tagLine}
                </span>
              </p>
            </div>
            <ArrowRight className="ml-4 text-text-muted group-hover:text-accent transition-colors" />
          </button>
        </div>
      )}

      {!detectedAccount && (
        <div className="grid grid-cols-3 gap-6 mt-12 text-center max-w-3xl">
          <div className="p-6 rounded-2xl bg-bg-card/30 border border-border/30">
            <Activity className="w-8 h-8 text-accent mx-auto mb-3" />
            <h3 className="font-medium text-text-primary mb-1">Live Аналитика</h3>
            <p className="text-sm text-text-muted">Отслеживайте текущие матчи в реальном времени</p>
          </div>
          <div className="p-6 rounded-2xl bg-bg-card/30 border border-border/30">
            <Trophy className="w-8 h-8 text-gold mx-auto mb-3" />
            <h3 className="font-medium text-text-primary mb-1">Детальная Статистика</h3>
            <p className="text-sm text-text-muted">Изучайте историю игр и винрейт чемпионов</p>
          </div>
          <div className="p-6 rounded-2xl bg-bg-card/30 border border-border/30">
            <Eye className="w-8 h-8 text-win mx-auto mb-3" />
            <h3 className="font-medium text-text-primary mb-1">AI Тренер</h3>
            <p className="text-sm text-text-muted">Получайте советы от искусственного интеллекта</p>
          </div>
        </div>
      )}
    </div>
  );
}
