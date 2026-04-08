import { SearchBar } from "../SearchBar";
import { ChevronRight } from "lucide-react";
import type { DetectedAccount } from "../../lib/types";

interface Props {
  detectedAccount: DetectedAccount | null;
  onSearch: (gameName: string, tagLine: string) => void;
  onBadgeClick: () => void;
  loading: boolean;
}

export function HomeVariant3({ detectedAccount, onSearch, onBadgeClick, loading }: Props) {
  return (
    <div className="relative min-h-[80vh] flex flex-col items-center justify-center py-24 -mt-6">
      {/* Immersive Background */}
      <div
        className="absolute inset-0 z-0 opacity-20 pointer-events-none"
        style={{
          backgroundImage: "url('https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg')",
          backgroundSize: "cover",
          backgroundPosition: "center",
          filter: "blur(8px) brightness(0.5)",
          maskImage: "linear-gradient(to bottom, transparent, black 20%, black 80%, transparent)",
          WebkitMaskImage: "linear-gradient(to bottom, transparent, black 20%, black 80%, transparent)",
        }}
      />

      <div className="relative z-10 w-full max-w-3xl px-4 flex flex-col items-center">
        <div className="mb-12 text-center animate-in fade-in slide-in-from-top-8 duration-700">
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-bg-card/40 backdrop-blur-md border border-white/10 text-white/80 text-sm font-medium mb-6">
            <span className="w-2 h-2 rounded-full bg-accent animate-pulse" />
            LeagueEye v1.0
          </div>
          <h1 className="text-6xl md:text-7xl font-extrabold text-transparent bg-clip-text bg-gradient-to-r from-white via-white/90 to-white/50 tracking-tighter mb-6">
            Доминируй в ущелье
          </h1>
          <p className="text-xl text-white/60 max-w-xl mx-auto font-light">
            Продвинутая статистика, анализ матчей и советы от AI в реальном времени.
          </p>
        </div>

        <div className="w-full max-w-2xl bg-bg-card/40 backdrop-blur-xl p-2 rounded-2xl border border-white/10 shadow-2xl mb-12 animate-in fade-in slide-in-from-bottom-8 duration-700 delay-150">
          <SearchBar onSearch={onSearch} loading={loading} />
        </div>

        {detectedAccount && (
          <div className="animate-in fade-in zoom-in-95 duration-700 delay-300">
            <button
              onClick={onBadgeClick}
              className="group relative overflow-hidden rounded-2xl p-[1px] transition-all hover:scale-105"
            >
              <div className="absolute inset-0 bg-gradient-to-r from-accent via-purple-500 to-accent opacity-50 group-hover:opacity-100 transition-opacity" />
              <div className="relative flex items-center gap-4 bg-bg-card/90 backdrop-blur-sm px-6 py-4 rounded-2xl">
                <img
                  src={`https://ddragon.leagueoflegends.com/cdn/14.8.1/img/profileicon/${detectedAccount.profileIconId}.png`}
                  alt="Profile"
                  className="w-12 h-12 rounded-full ring-2 ring-white/20"
                />
                <div className="text-left">
                  <p className="text-sm font-medium text-white/60">Ваш профиль</p>
                  <p className="text-lg font-bold text-white">
                    {detectedAccount.gameName}
                    <span className="text-white/40 font-normal ml-1">
                      #{detectedAccount.tagLine}
                    </span>
                  </p>
                </div>
                <ChevronRight className="ml-4 text-white/40 group-hover:text-white transition-colors" />
              </div>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
