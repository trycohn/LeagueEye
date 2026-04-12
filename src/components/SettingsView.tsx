import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Settings,
  Download,
  CheckCircle,
  AlertCircle,
  Loader2,
  RefreshCw,
  Wifi,
  WifiOff,
  Info,
} from "lucide-react";
import { useUpdater } from "../hooks/useUpdater";

export function SettingsView() {
  const {
    version,
    status,
    updateInfo,
    error,
    checkForUpdate,
    installUpdate,
    dismissUpdate,
  } = useUpdater();

  const [serverOnline, setServerOnline] = useState<boolean | null>(null);

  // Check server connectivity
  useEffect(() => {
    let mounted = true;

    async function checkServer() {
      try {
        await invoke("get_global_dashboard");
        if (mounted) setServerOnline(true);
      } catch {
        if (mounted) setServerOnline(false);
      }
    }

    checkServer();
    const interval = setInterval(checkServer, 30_000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="max-w-2xl mx-auto py-6">
      {/* Header */}
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-sm bg-[#1e2130] border border-[#2a2d3a] flex items-center justify-center text-accent">
          <Settings size={20} />
        </div>
        <div>
          <h1 className="text-lg font-bold text-[#e2e8f0]">Настройки</h1>
          <p className="text-xs text-[#64748b]">
            Управление приложением
          </p>
        </div>
      </div>

      {/* About */}
      <Section title="О программе" icon={<Info size={16} className="text-[#3b82f6]" />}>
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-bold text-[#e2e8f0]">LeagueEye</div>
            <div className="text-xs text-[#64748b] mt-0.5">
              Анализ статистики League of Legends
            </div>
          </div>
          <div className="text-sm font-mono text-[#94a3b8] bg-[#1e2130] px-3 py-1 rounded-sm border border-[#2a2d3a]">
            v{version || "..."}
          </div>
        </div>
      </Section>

      {/* Updates */}
      <Section title="Обновления" icon={<Download size={16} className="text-[#22c55e]" />}>
        <div className="space-y-4">
          {/* Current version + check button */}
          <div className="flex items-center justify-between">
            <div className="text-sm text-[#94a3b8]">
              Текущая версия:{" "}
              <span className="font-mono font-bold text-[#e2e8f0]">
                {version || "..."}
              </span>
            </div>
            <button
              onClick={checkForUpdate}
              disabled={status === "checking" || status === "downloading"}
              className="flex items-center gap-2 px-3 py-1.5 text-xs font-semibold rounded-sm bg-[#1e2130] border border-[#2a2d3a] text-[#e2e8f0] hover:bg-[#252838] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {status === "checking" ? (
                <>
                  <Loader2 size={14} className="animate-spin" />
                  Проверка...
                </>
              ) : (
                <>
                  <RefreshCw size={14} />
                  Проверить обновления
                </>
              )}
            </button>
          </div>

          {/* Status messages */}
          {status === "up-to-date" && (
            <div className="flex items-center gap-2 text-sm text-[#22c55e] bg-[#22c55e]/10 border border-[#22c55e]/20 rounded-sm px-3 py-2">
              <CheckCircle size={16} />
              У вас установлена последняя версия
            </div>
          )}

          {status === "error" && error && (
            <div className="flex items-center gap-2 text-sm text-[#ef4444] bg-[#ef4444]/10 border border-[#ef4444]/20 rounded-sm px-3 py-2">
              <AlertCircle size={16} />
              {error}
            </div>
          )}

          {/* Update available */}
          {status === "available" && updateInfo && (
            <div className="bg-[#1e2130] border border-[#3b82f6]/30 rounded-sm p-4 space-y-3">
              <div className="flex items-center gap-2">
                <Download size={16} className="text-[#3b82f6]" />
                <span className="text-sm font-bold text-[#e2e8f0]">
                  Доступна версия {updateInfo.version}
                </span>
              </div>

              {updateInfo.body && (
                <div className="text-xs text-[#94a3b8] leading-relaxed whitespace-pre-wrap">
                  {updateInfo.body}
                </div>
              )}

              <div className="flex items-center gap-2">
                <button
                  onClick={installUpdate}
                  className="flex items-center gap-2 px-4 py-2 text-xs font-bold rounded-sm bg-[#3b82f6] text-white hover:bg-[#2563eb] transition-colors"
                >
                  <Download size={14} />
                  Установить и перезапустить
                </button>
                <button
                  onClick={dismissUpdate}
                  className="px-4 py-2 text-xs font-semibold rounded-sm bg-[#1a1d28] border border-[#2a2d3a] text-[#94a3b8] hover:text-[#e2e8f0] hover:bg-[#252838] transition-colors"
                >
                  Позже
                </button>
              </div>
            </div>
          )}

          {/* Downloading */}
          {status === "downloading" && (
            <div className="flex items-center gap-3 text-sm text-[#3b82f6] bg-[#3b82f6]/10 border border-[#3b82f6]/20 rounded-sm px-3 py-3">
              <Loader2 size={16} className="animate-spin" />
              <div>
                <div className="font-semibold">Скачивание и установка...</div>
                <div className="text-xs text-[#64748b] mt-0.5">
                  Приложение перезапустится автоматически
                </div>
              </div>
            </div>
          )}
        </div>
      </Section>

      {/* Connection */}
      <Section
        title="Подключение"
        icon={
          serverOnline ? (
            <Wifi size={16} className="text-[#22c55e]" />
          ) : (
            <WifiOff size={16} className="text-[#ef4444]" />
          )
        }
      >
        <div className="flex items-center gap-3">
          <div
            className={`w-2.5 h-2.5 rounded-full ${
              serverOnline === null
                ? "bg-[#64748b] animate-pulse"
                : serverOnline
                  ? "bg-[#22c55e]"
                  : "bg-[#ef4444]"
            }`}
          />
          <span className="text-sm text-[#e2e8f0]">
            {serverOnline === null
              ? "Проверка подключения..."
              : serverOnline
                ? "Сервер подключён"
                : "Сервер недоступен"}
          </span>
        </div>
      </Section>
    </div>
  );
}

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm mb-4">
      <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
        {icon}
        <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">
          {title}
        </h2>
      </div>
      <div className="px-4 py-4">{children}</div>
    </div>
  );
}
