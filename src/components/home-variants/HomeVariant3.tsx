import { Activity, Server, AlertTriangle, CheckCircle2 } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

export function HomeVariant3({ detectedAccount, onBadgeClick }: Props) {
  return (
    <div className="max-w-4xl mx-auto py-12">
      {/* Account Banner (if connected) */}
      {detectedAccount ? (
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm shadow-md">
          <div className="px-6 py-4 border-b border-[#2a2d3a] flex items-center justify-between bg-[#1e2130]">
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider flex items-center gap-2">
              <Activity size={16} className="text-[#3b82f6]" />
              Аналитика профиля
            </h2>
            <div className="text-xs font-bold text-[#22c55e] uppercase tracking-wider flex items-center gap-1.5">
              <CheckCircle2 size={14} /> Синхронизировано
            </div>
          </div>
          <div className="p-6 grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="col-span-1 md:col-span-2 flex items-center gap-6">
              <img
                src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                alt="Profile"
                className="w-24 h-24 rounded-sm border-2 border-[#2a2d3a]"
              />
              <div>
                <div className="text-xs text-[#94a3b8] font-bold uppercase tracking-wider mb-1">Уровень {detectedAccount.summonerLevel}</div>
                <div className="text-3xl font-black text-[#e2e8f0] mb-2">
                  {detectedAccount.gameName} <span className="text-[#64748b] font-normal">#{detectedAccount.tagLine}</span>
                </div>
                <button
                  onClick={onBadgeClick}
                  className="px-6 py-2 bg-[#3b82f6] hover:bg-[#2563eb] text-white text-sm font-bold rounded-sm transition-colors"
                >
                  Открыть полную статистику
                </button>
              </div>
            </div>
            <div className="col-span-1 border-l border-[#2a2d3a] pl-6 flex flex-col justify-center">
              <div className="text-xs font-bold text-[#94a3b8] uppercase tracking-wider mb-2">Последние 20 игр (Mock)</div>
              <div className="flex items-end gap-1 h-16 w-full">
                {[...Array(20)].map((_, i) => (
                  <div
                    key={i}
                    className={`w-full rounded-t-sm ${Math.random() > 0.4 ? 'bg-[#3b82f6]' : 'bg-[#ef4444]'}`}
                    style={{ height: `${Math.random() * 60 + 40}%` }}
                  ></div>
                ))}
              </div>
              <div className="mt-2 flex justify-between text-xs font-bold">
                <span className="text-[#3b82f6]">12W</span>
                <span className="text-[#ef4444]">8L</span>
                <span className="text-[#e2e8f0]">60%</span>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm shadow-md">
          <div className="px-6 py-4 border-b border-[#2a2d3a] flex items-center justify-between bg-[#1e2130]">
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider flex items-center gap-2">
              <Server size={16} className="text-[#ef4444]" />
              Статус клиента
            </h2>
            <div className="text-xs font-bold text-[#ef4444] uppercase tracking-wider flex items-center gap-1.5 animate-pulse">
              <AlertTriangle size={14} /> Ожидание
            </div>
          </div>
          <div className="p-12 text-center flex flex-col items-center justify-center">
            <div className="w-16 h-16 rounded-full bg-[#1e2130] border border-[#2a2d3a] flex items-center justify-center mb-6">
              <Server size={32} className="text-[#64748b]" />
            </div>
            <h3 className="text-xl font-bold text-[#e2e8f0] mb-2">League of Legends не запущен</h3>
            <p className="text-[#94a3b8] max-w-md mx-auto mb-8 text-sm leading-relaxed">
              Запустите клиент игры, чтобы LeagueEye мог автоматически определить ваш профиль, загрузить историю матчей и активировать Live Tracker во время игры.
            </p>
            <div className="bg-[#1e2130] border border-[#2a2d3a] rounded-sm px-6 py-4 text-left w-full max-w-sm">
              <div className="text-xs font-bold text-[#e2e8f0] uppercase tracking-wider mb-3">Инструкция:</div>
              <ol className="list-decimal list-inside text-sm text-[#94a3b8] space-y-2">
                <li>Откройте Riot Client</li>
                <li>Запустите League of Legends</li>
                <li>Авторизуйтесь в свой аккаунт</li>
                <li>LeagueEye автоматически подключится</li>
              </ol>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
