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
  const resetProfileState = useCallback(() => {
    setProfile(null);
    setMastery([]);
    setMatches([]);
    setChampionStats([]);
    setHasMore(false);
    setTotalCached(0);
    setTotalWins(0);
    setTotalLosses(0);
  }, []);

  const loadProfileByPuuid = useCallback(
    async (p: PlayerProfile) => {
      const gen = ++genRef.current;
      setCurrentPuuid(p.puuid);

      const [matchesAndStats, masteryData] = await Promise.all([
        invoke<MatchesAndStats>("get_matches_and_stats", { puuid: p.puuid }),
        invoke<MasteryInfo[]>("get_mastery", { puuid: p.puuid }).catch(() => [] as MasteryInfo[]),
      ]);

      if (gen !== genRef.current) return;

      setProfile(p);
      setMatches(matchesAndStats.matches);
      setChampionStats(matchesAndStats.championStats);
      setTotalCached(matchesAndStats.totalCached);
      setTotalWins(matchesAndStats.totalWins);
      setTotalLosses(matchesAndStats.totalLosses);
      setHasMore(matchesAndStats.matches.length < matchesAndStats.totalCached);
      setMastery(masteryData);
    },
    []
  );

  const lastSearchRef = useRef<{ gameName: string; tagLine: string; profile: PlayerProfile } | null>(null);

  const getCachedSearchProfile = useCallback((gameName: string, tagLine: string) => {
    const nameNorm = gameName.toLowerCase();
    const tagNorm = tagLine.toLowerCase();
    const cached = lastSearchRef.current;

    if (!cached) {
      return null;
    }

    return cached.gameName.toLowerCase() === nameNorm &&
      cached.tagLine.toLowerCase() === tagNorm
      ? cached.profile
      : null;
  }, []);

  const fetchProfile = useCallback(
    async (gameName: string, tagLine: string) => {
      const cached = getCachedSearchProfile(gameName, tagLine);
      if (cached) {
        return cached;
      }

      const p = await invoke<PlayerProfile>("search_player", {
        gameName,
        tagLine,
      });
      lastSearchRef.current = { gameName, tagLine, profile: p };
      return p;
    },
    [getCachedSearchProfile]
  );

  const buildDetectedProfile = useCallback(
    (detected: DetectedAccount): PlayerProfile => ({
      puuid: detected.puuid,
      gameName: detected.gameName,
      tagLine: detected.tagLine,
      summonerLevel: detected.summonerLevel,
      profileIconId: detected.profileIconId,
      ranked: detected.ranked,
    }),
    []
  );

  const searchPlayer = useCallback(
    async (gameName: string, tagLine: string) => {
      if (busyRef.current) return;

      const cachedProfile = getCachedSearchProfile(gameName, tagLine);
      if (cachedProfile) {
        busyRef.current = true;
        setLoading(true);
        setError(null);
        try {
          await loadProfileByPuuid(cachedProfile);
        } finally {
          setLoading(false);
          busyRef.current = false;
        }
        return;
      }

      busyRef.current = true;

      setLoading(true);
      setError(null);
      resetProfileState();

      try {
        const p = await fetchProfile(gameName, tagLine);
        await loadProfileByPuuid(p);
      } catch (e) {
        setError(typeof e === "string" ? e : String(e));
      } finally {
        setLoading(false);
        busyRef.current = false;
      }
    },
    [fetchProfile, getCachedSearchProfile, loadProfileByPuuid, resetProfileState]
  );

  const loadDetectedAccount = useCallback(
    async (detected: DetectedAccount) => {
      if (busyRef.current) return;
      busyRef.current = true;

      setLoading(true);
      setError(null);
      resetProfileState();

      try {
        let p: PlayerProfile;

        try {
          p = await fetchProfile(detected.gameName, detected.tagLine);
        } catch {
          // Cached account keeps the profile usable when name/tag lookup fails.
          p = buildDetectedProfile(detected);
        }

        await loadProfileByPuuid(p);
      } catch (e) {
        setError(typeof e === "string" ? e : String(e));
      } finally {
        setLoading(false);
        busyRef.current = false;
      }
    },
    [buildDetectedProfile, fetchProfile, loadProfileByPuuid, resetProfileState]
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
      setMatches((prev: MatchSummary[]) => {
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

  // Load matches up to a target count in one shot (used by PlayerTrends).
  // `target` of 0 means load all available (up to totalCached).
  const loadMatchesUpTo = useCallback(async (target: number) => {
    if (!currentPuuid) return;
    const needed = target === 0 ? totalCached : target;
    const currentLen = matches.length;
    if (currentLen >= needed) return;

    const remaining = needed - currentLen;
    setLoadingMore(true);
    try {
      const newMatches = await invoke<MatchSummary[]>("load_more_matches", {
        puuid: currentPuuid,
        offset: currentLen,
        limit: remaining,
      });
      if (newMatches.length === 0) {
        setHasMore(false);
        return;
      }
      setMatches((prev: MatchSummary[]) => {
        const combined = [...prev, ...newMatches];
        setHasMore(combined.length < totalCached);
        return combined;
      });
    } catch (e) {
      console.error("loadMatchesUpTo error:", e);
    } finally {
      setLoadingMore(false);
    }
  }, [currentPuuid, matches.length, totalCached]);

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
    loadMatchesUpTo,
  };
}
