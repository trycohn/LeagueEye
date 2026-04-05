import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  PlayerProfile,
  MasteryInfo,
  MatchSummary,
  ChampionStat,
  DetectedAccount,
  MatchesAndStats,
} from "../lib/types";

const PAGE_SIZE = 15;

export function useRiotApi() {
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [mastery, setMastery] = useState<MasteryInfo[]>([]);
  const [matches, setMatches] = useState<MatchSummary[]>([]);
  const [championStats, setChampionStats] = useState<ChampionStat[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [totalCached, setTotalCached] = useState(0);
  const [totalWins, setTotalWins] = useState(0);
  const [totalLosses, setTotalLosses] = useState(0);
  const [currentPuuid, setCurrentPuuid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const genRef = useRef(0);
  const busyRef = useRef(false);

  const loadProfileByPuuid = useCallback(
    async (p: PlayerProfile) => {
      const gen = ++genRef.current;
      setProfile(p);
      setCurrentPuuid(p.puuid);

      const masteryPromise = invoke<MasteryInfo[]>("get_mastery", { puuid: p.puuid });
      const matchesAndStats = await invoke<MatchesAndStats>("get_matches_and_stats", { puuid: p.puuid });

      if (gen !== genRef.current) return;

      setMatches(matchesAndStats.matches);
      setChampionStats(matchesAndStats.championStats);
      setTotalCached(matchesAndStats.totalCached);
      setTotalWins(matchesAndStats.totalWins);
      setTotalLosses(matchesAndStats.totalLosses);
      setHasMore(matchesAndStats.matches.length < matchesAndStats.totalCached);

      masteryPromise.then((masteryData) => {
        if (gen !== genRef.current) return;
        setMastery(masteryData);
      }).catch(() => {});
    },
    []
  );

  const lastSearchRef = useRef<{ gameName: string; tagLine: string; profile: PlayerProfile } | null>(null);

  const searchPlayer = useCallback(
    async (gameName: string, tagLine: string) => {
      if (busyRef.current) return;

      const nameNorm = gameName.toLowerCase();
      const tagNorm = tagLine.toLowerCase();
      const cached = lastSearchRef.current;
      if (cached && cached.gameName.toLowerCase() === nameNorm && cached.tagLine.toLowerCase() === tagNorm) {
        busyRef.current = true;
        setLoading(true);
        setError(null);
        try {
          await loadProfileByPuuid(cached.profile);
        } finally {
          setLoading(false);
          busyRef.current = false;
        }
        return;
      }

      busyRef.current = true;

      setLoading(true);
      setError(null);
      setProfile(null);
      setMastery([]);
      setMatches([]);
      setChampionStats([]);
      setHasMore(false);
      setTotalCached(0);
      setTotalWins(0);
      setTotalLosses(0);

      try {
        const p = await invoke<PlayerProfile>("search_player", {
          gameName,
          tagLine,
        });
        lastSearchRef.current = { gameName, tagLine, profile: p };
        await loadProfileByPuuid(p);
      } catch (e) {
        setError(typeof e === "string" ? e : String(e));
      } finally {
        setLoading(false);
        busyRef.current = false;
      }
    },
    [loadProfileByPuuid]
  );

  const loadDetectedAccount = useCallback(
    async (detected: DetectedAccount) => {
      if (busyRef.current) return;
      busyRef.current = true;

      setLoading(true);
      setError(null);
      setProfile(null);
      setMastery([]);
      setMatches([]);
      setChampionStats([]);
      setHasMore(false);
      setTotalCached(0);
      setTotalWins(0);
      setTotalLosses(0);

      try {
        const p: PlayerProfile = {
          puuid: detected.puuid,
          gameName: detected.gameName,
          tagLine: detected.tagLine,
          summonerLevel: detected.summonerLevel,
          profileIconId: detected.profileIconId,
          ranked: detected.ranked,
        };
        await loadProfileByPuuid(p);
      } catch (e) {
        setError(typeof e === "string" ? e : String(e));
      } finally {
        setLoading(false);
        busyRef.current = false;
      }
    },
    [loadProfileByPuuid]
  );

  const loadMoreMatches = useCallback(async () => {
    if (!currentPuuid || loadingMore) return;
    setLoadingMore(true);
    try {
      const offset = matches.length;
      const newMatches = await invoke<MatchSummary[]>("load_more_matches", {
        puuid: currentPuuid,
        offset,
        limit: PAGE_SIZE,
      });
      if (newMatches.length === 0) {
        setHasMore(false);
        return;
      }
      setMatches((prev) => {
        const combined = [...prev, ...newMatches];
        setHasMore(combined.length < totalCached);
        return combined;
      });
    } catch (e) {
      console.error("loadMoreMatches error:", e);
    } finally {
      setLoadingMore(false);
    }
  }, [currentPuuid, matches.length, loadingMore, totalCached]);

  return {
    profile,
    mastery,
    matches,
    championStats,
    loading,
    loadingMore,
    hasMore,
    totalCached,
    totalWins,
    totalLosses,
    error,
    searchPlayer,
    loadDetectedAccount,
    loadMoreMatches,
  };
}
