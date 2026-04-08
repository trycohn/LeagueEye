import { LineChart, TrendingUp, Crosshair, Shield } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

// Mock data representing aggregated stats from `matches` and `rank_snapshots` tables
const RECENT_FORM = {
  winrate: 65,
  matches: 20,
  kda: "3.4",
  kills: 7.2,
  deaths: 4.1,
  assists: 6.8,
  csPerMin: 7.8,
  visionScore: 24,
};

const LP_HISTORY = [
  { day: "Пн", lp: 240 },
  { day: "Вт", lp: 225 },
  { day: "Ср", lp: 260 },
  { day: "Чт", lp: 285 },
  { day: "Пт", lp: 270 },
  { day: "Сб", lp: 310 },
  { day: "Вс", lp: 345 },
];

const HIGHLIGHTS = [
  { type: "penta", text: "Пента-килл на Jinx", date: "2 дня назад" },
  { type: "damage", text: "Рекорд урона: 64,320 (Azir)", date: "4 дня назад" },
  { type: "vision", text: "Идеальный вижн: 85 очков (Thresh)", date: "Неделю назад" },
];

export function HomeVariant4({ detectedAccount, onBadgeClick }: Props) {
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
              <div className="text-xs text-[#94a3b8] font-semibold uppercase tracking-wider mb-0.5">Трекинг активен</div>
              <div className="text-base font-bold text-[#e2e8f0]">
                {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
              </div>
            </div>
          </div>
          <button
            onClick={onBadgeClick}
            className="px-4 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors"
          >
            Подробная статистика
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* LP Tracker (from rank_snapshots) */}
        <div className="lg:col-span-2 bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <LineChart size={16} className="text-[#3b82f6]" />
              <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Прогресс LP (Последние 7 дней)</h2>
            </div>
            <div className="text-xs font-bold text-[#22c55e]">+105 LP</div>
          </div>
          <div className="p-6 flex-1 flex items-end gap-2 h-48">
            {LP_HISTORY.map((point, i) => {
              const maxLp = Math.max(...LP_HISTORY.map(p => p.lp));
              const minLp = Math.min(...LP_HISTORY.map(p => p.lp));
              const height = ((point.lp - minLp + 50) / (maxLp - minLp + 50)) * 100;
              
              return (
                <div key={i} className="flex-1 flex flex-col items-center gap-2 group">
                  <div className="w-full flex items-end justify-center h-full relative">
                    <div 
                      className="w-full bg-[#3b82f6]/20 border-t-2 border-[#3b82f6] rounded-t-sm group-hover:bg-[#3b82f6]/40 transition-colors relative"
                      style={{ height: `${height}%` }}
                    >
                      <div className="absolute -top-6 left-1/2 -translate-x-1/2 text-xs font-bold text-[#e2e8f0] opacity-0 group-hover:opacity-100 transition-opacity">
                        {point.lp}
                      </div>
                    </div>
                  </div>
                  <div className="text-xs font-bold text-[#64748b]">{point.day}</div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Recent Form (from matches) */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm flex flex-col">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <TrendingUp size={16} className="text-[#eab308]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">Форма (20 игр)</h2>
          </div>
          <div className="p-4 flex-1 flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <div className="text-sm font-bold text-[#94a3b8]">Винрейт</div>
              <div className="text-lg font-black text-[#22c55e]">{RECENT_FORM.winrate}%</div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm font-bold text-[#94a3b8]">KDA</div>
              <div className="text-right">
                <div className="text-base font-black text-[#e2e8f0]">{RECENT_FORM.kda}:1</div>
                <div className="text-xs font-medium text-[#64748b]">{RECENT_FORM.kills} / {RECENT_FORM.deaths} / {RECENT_FORM.assists}</div>
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm font-bold text-[#94a3b8]">CS / Мин</div>
              <div className="text-base font-black text-[#e2e8f0]">{RECENT_FORM.csPerMin}</div>
            </div>
            
            <div className="mt-auto pt-4 border-t border-[#2a2d3a]">
              <div className="text-xs font-bold text-[#64748b] uppercase tracking-wider mb-3">Хайлайты</div>
              <div className="space-y-2">
                {HIGHLIGHTS.map((h, i) => (
                  <div key={i} className="flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2 font-medium text-[#e2e8f0]">
                      {h.type === 'penta' ? <Crosshair size={12} className="text-[#ef4444]" /> : <Shield size={12} className="text-[#eab308]" />}
                      {h.text}
                    </div>
                    <div className="text-[#64748b]">{h.date}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
