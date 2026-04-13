import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronLeft, Loader2, Search } from "lucide-react";
import { profileIconUrl } from "../lib/ddragon";
import { RankBadge } from "./RankBadge";
import type {
  PlayerProfile,
  MatchesAndStats,
  FavoritePlayer,
} from "../lib/types";

interface Props {
  leftProfile: PlayerProfile;
  leftStats: { totalWins: number; totalLosses: number; matches: { kills: number; deaths: number; assists: number; cs: number; gameDuration: number }[] };
  favorites: FavoritePlayer[];
  onBack: () => void;
}

interface CompareStats {
  winrate: number;
  avgKda: number;
  avgKills: number;
  avgDeaths: number;
  avgAssists: number;
  avgCsPerMin: number;
  totalGames: number;
}

function computeStats(data: {
  totalWins: number;
  totalLosses: number;
  matches: { kills: number; deaths: number; assists: number; cs: number; gameDuration: number }[];
}): CompareStats {
  const total = data.totalWins + data.totalLosses;
  const winrate = total > 0 ? (data.totalWins / total) * 100 : 0;

  const matches = data.matches;
  const n = matches.length || 1;
  const avgKills = matches.reduce((s, m) => s + m.kills, 0) / n;
  const avgDeaths = matches.reduce((s, m) => s + m.deaths, 0) / n;
  const avgAssists = matches.reduce((s, m) => s + m.assists, 0) / n;
  const avgCsPerMin =
    matches.reduce((s, m) => s + (m.gameDuration > 0 ? (m.cs / m.gameDuration) * 60 : 0), 0) / n;
  const avgKda = avgDeaths > 0 ? (avgKills + avgAssists) / avgDeaths : avgKills + avgAssists;

  return { winrate, avgKda, avgKills, avgDeaths, avgAssists, avgCsPerMin, totalGames: total };
}

function CompareBar({
  label,
  leftVal,
  rightVal,
  format,
  higherIsBetter = true,
}: {
  label: string;
  leftVal: number;
  rightVal: number;
  format: (v: number) => string;
  higherIsBetter?: boolean;
}) {
  const max = Math.max(leftVal, rightVal, 0.01);
  const leftPct = (leftVal / max) * 100;
  const rightPct = (rightVal / max) * 100;

  const leftBetter = higherIsBetter ? leftVal > rightVal : leftVal < rightVal;
  const rightBetter = higherIsBetter ? rightVal > leftVal : rightVal < leftVal;

  return (
    <div className="flex flex-col gap-1">
      <div className="text-center text-xs font-semibold text-[#94a3b8] uppercase tracking-wider">
        {label}
      </div>
      <div className="flex items-center gap-3">
        <span
          className={`w-16 text-right text-sm font-bold ${
            leftBetter ? "text-[#22c55e]" : "text-[#e2e8f0]"
          }`}
        >
          {format(leftVal)}
        </span>
        <div className="flex-1 flex gap-1 h-5">
          <div className="flex-1 flex justify-end">
            <div
              className={`h-full rounded-l-sm ${leftBetter ? "bg-[#22c55e]/60" : "bg-[#475569]/40"}`}
              style={{ width: `${leftPct}%` }}
            />
          </div>
          <div className="flex-1">
            <div
              className={`h-full rounded-r-sm ${rightBetter ? "bg-[#22c55e]/60" : "bg-[#475569]/40"}`}
              style={{ width: `${rightPct}%` }}
            />
          </div>
        </div>
        <span
          className={`w-16 text-left text-sm font-bold ${
            rightBetter ? "text-[#22c55e]" : "text-[#e2e8f0]"
          }`}
        >
          {format(rightVal)}
        </span>
      </div>
    </div>
  );
}

export function CompareView({ leftProfile, leftStats, favorites, onBack }: Props) {
  const [rightProfile, setRightProfile] = useState<PlayerProfile | null>(null);
  const [rightMatchData, setRightMatchData] = useState<{
    totalWins: number;
    totalLosses: number;
    matches: { kills: number; deaths: number; assists: number; cs: number; gameDuration: number }[];
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [searchInput, setSearchInput] = useState("");

  const leftComputed = computeStats(leftStats);

  async function loadRight(gameName: string, tagLine: string) {
    setLoading(true);
    try {
      const profile = await invoke<PlayerProfile>("search_player", { gameName, tagLine });
      setRightProfile(profile);

      const data = await invoke<MatchesAndStats>("get_matches_and_stats", { puuid: profile.puuid });
      setRightMatchData({
        totalWins: data.totalWins,
        totalLosses: data.totalLosses,
        matches: data.matches,
      });
    } catch (e) {
      console.error("CompareView loadRight error:", e);
    } finally {
      setLoading(false);
    }
  }

  function handleSearchSubmit(e: React.FormEvent) {
    e.preventDefault();
    const parts = searchInput.split("#");
    if (parts.length === 2 && parts[0].trim() && parts[1].trim()) {
      loadRight(parts[0].trim(), parts[1].trim());
    }
  }

  const rightComputed = rightMatchData ? computeStats(rightMatchData) : null;

  const soloRankLeft = leftProfile.ranked.find((r) => r.queueType === "RANKED_SOLO_5x5");
  const soloRankRight = rightProfile?.ranked.find((r) => r.queueType === "RANKED_SOLO_5x5");

  return (
    <div className="flex flex-col gap-5">
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-text-muted hover:text-text-primary text-sm transition-colors w-fit"
      >
        <ChevronLeft size={16} />
        Назад
      </button>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left Player */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-5">
          <div className="flex items-center gap-3 mb-3">
            <img
              src={profileIconUrl(leftProfile.profileIconId)}
              alt={leftProfile.gameName}
              className="w-14 h-14 rounded-sm border-2 border-[#2a2d3a]"
            />
            <div>
              <div className="text-lg font-bold text-[#e2e8f0]">
                {leftProfile.gameName}
                <span className="text-[#64748b] font-normal ml-1">#{leftProfile.tagLine}</span>
              </div>
              {soloRankLeft && <RankBadge rank={soloRankLeft} />}
            </div>
          </div>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div className="text-[#64748b]">Игр</div>
            <div className="text-[#e2e8f0] font-semibold">{leftComputed.totalGames}</div>
            <div className="text-[#64748b]">Винрейт</div>
            <div className="text-[#e2e8f0] font-semibold">{leftComputed.winrate.toFixed(1)}%</div>
            <div className="text-[#64748b]">KDA</div>
            <div className="text-[#e2e8f0] font-semibold">{leftComputed.avgKda.toFixed(2)}</div>
            <div className="text-[#64748b]">CS/мин</div>
            <div className="text-[#e2e8f0] font-semibold">{leftComputed.avgCsPerMin.toFixed(1)}</div>
          </div>
        </div>

        {/* Right Player */}
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-5">
          {rightProfile && rightComputed ? (
            <>
              <div className="flex items-center gap-3 mb-3">
                <img
                  src={profileIconUrl(rightProfile.profileIconId)}
                  alt={rightProfile.gameName}
                  className="w-14 h-14 rounded-sm border-2 border-[#2a2d3a]"
                />
                <div>
                  <div className="text-lg font-bold text-[#e2e8f0]">
                    {rightProfile.gameName}
                    <span className="text-[#64748b] font-normal ml-1">#{rightProfile.tagLine}</span>
                  </div>
                  {soloRankRight && <RankBadge rank={soloRankRight} />}
                </div>
              </div>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <div className="text-[#64748b]">Игр</div>
                <div className="text-[#e2e8f0] font-semibold">{rightComputed.totalGames}</div>
                <div className="text-[#64748b]">Винрейт</div>
                <div className="text-[#e2e8f0] font-semibold">{rightComputed.winrate.toFixed(1)}%</div>
                <div className="text-[#64748b]">KDA</div>
                <div className="text-[#e2e8f0] font-semibold">{rightComputed.avgKda.toFixed(2)}</div>
                <div className="text-[#64748b]">CS/мин</div>
                <div className="text-[#e2e8f0] font-semibold">{rightComputed.avgCsPerMin.toFixed(1)}</div>
              </div>
            </>
          ) : loading ? (
            <div className="flex flex-col items-center justify-center py-12 gap-3">
              <Loader2 size={28} className="animate-spin text-accent" />
              <p className="text-[#64748b] text-sm">Загрузка...</p>
            </div>
          ) : (
            <div className="flex flex-col gap-4 py-4">
              <p className="text-[#64748b] text-sm text-center">Выберите игрока для сравнения</p>

              {/* Search */}
              <form onSubmit={handleSearchSubmit} className="flex gap-2">
                <input
                  type="text"
                  value={searchInput}
                  onChange={(e) => setSearchInput(e.target.value)}
                  placeholder="Имя#Тег"
                  className="flex-1 px-3 py-2 rounded-sm bg-[#1e2130] border border-[#2a2d3a] text-sm text-[#e2e8f0] placeholder-[#64748b] focus:outline-none focus:border-accent"
                />
                <button
                  type="submit"
                  className="px-3 py-2 rounded-sm bg-accent/20 text-accent hover:bg-accent/30 transition-colors"
                >
                  <Search size={16} />
                </button>
              </form>

              {/* Quick pick from favorites */}
              {favorites.length > 0 && (
                <div className="flex flex-col gap-2">
                  <div className="text-xs font-semibold text-[#64748b] uppercase tracking-wider">
                    Из избранных
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {favorites.map((fav) => (
                      <button
                        key={fav.puuid}
                        onClick={() => loadRight(fav.gameName, fav.tagLine)}
                        className="flex items-center gap-1.5 px-2 py-1 bg-[#1e2130] border border-[#2a2d3a] rounded-sm text-xs text-[#e2e8f0] hover:border-[#3b82f6]/50 transition-colors"
                      >
                        <img
                          src={profileIconUrl(fav.profileIconId)}
                          alt={fav.gameName}
                          className="w-5 h-5 rounded-sm"
                        />
                        {fav.gameName}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Comparison bars */}
      {rightComputed && (
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm p-5 flex flex-col gap-4">
          <h3 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider text-center mb-2">
            Сравнение
          </h3>
          <CompareBar
            label="Винрейт"
            leftVal={leftComputed.winrate}
            rightVal={rightComputed.winrate}
            format={(v) => `${v.toFixed(1)}%`}
          />
          <CompareBar
            label="KDA"
            leftVal={leftComputed.avgKda}
            rightVal={rightComputed.avgKda}
            format={(v) => v.toFixed(2)}
          />
          <CompareBar
            label="Avg Kills"
            leftVal={leftComputed.avgKills}
            rightVal={rightComputed.avgKills}
            format={(v) => v.toFixed(1)}
          />
          <CompareBar
            label="Avg Deaths"
            leftVal={leftComputed.avgDeaths}
            rightVal={rightComputed.avgDeaths}
            format={(v) => v.toFixed(1)}
            higherIsBetter={false}
          />
          <CompareBar
            label="Avg Assists"
            leftVal={leftComputed.avgAssists}
            rightVal={rightComputed.avgAssists}
            format={(v) => v.toFixed(1)}
          />
          <CompareBar
            label="CS/мин"
            leftVal={leftComputed.avgCsPerMin}
            rightVal={rightComputed.avgCsPerMin}
            format={(v) => v.toFixed(1)}
          />
        </div>
      )}
    </div>
  );
}
