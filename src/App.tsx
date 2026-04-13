import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { SearchBar } from "./components/SearchBar";
import { ProfileCard } from "./components/ProfileCard";
import { MasteryList } from "./components/MasteryList";
import { MatchHistory } from "./components/MatchHistory";
import { ChampionStats } from "./components/ChampionStats";
import { PlayerTrends } from "./components/PlayerTrends";
import { AccountBadge } from "./components/AccountBadge";
import { LiveGameView } from "./components/LiveGameView";
import { useRiotApi } from "./hooks/useRiotApi";
import { useLiveGame } from "./hooks/useLiveGame";
import { useOverlayLifecycle } from "./hooks/useOverlayLifecycle";
import { useUpdater } from "./hooks/useUpdater";
import { HomeView } from "./components/HomeView";
import { SettingsView } from "./components/SettingsView";
import { Eye, AlertCircle, Loader2, ChevronLeft, Settings } from "lucide-react";
import type { DetectedAccount } from "./lib/types";

type View = "home" | "profile" | "live" | "settings";
type LeagueWindowVisibilityPayload = { visible: boolean };

const POLL_INTERVAL_MS = 4_000;

export default function App() {
  const {
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
  } = useRiotApi();

  const [view, setView] = useState<View>("home");
  const [prevView, setPrevView] = useState<View>("home");
  const [detectedAccount, setDetectedAccount] =
    useState<DetectedAccount | null>(null);
  const [clientOnline, setClientOnline] = useState(false);
  const [leagueWindowVisible, setLeagueWindowVisible] = useState(false);
  const [overlayRetryNonce, setOverlayRetryNonce] = useState(0);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const { liveData, phase } = useLiveGame(clientOnline);
  const overlayEligible = useOverlayLifecycle(clientOnline);
  const { updateAvailable } = useUpdater();
  const isLive = phase === "champ_select" || phase === "in_game";

  // Автопереключение на live view + отдельная политика видимости оверлея.
  const overlayShownRef = useRef(false);
  const overlayRetryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const leaveLiveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevPhaseRef = useRef<string>("none");

  // Сброс истории AI Coach при начале нового матча (none → champ_select)
  useEffect(() => {
    const prev = prevPhaseRef.current;
    prevPhaseRef.current = phase;

    if (prev === "none" && phase === "champ_select") {
      emit("coach-reset").catch(() => {});
    }
  }, [phase]);

  useEffect(() => {
    if (isLive) {
      // Cancel any pending "leave live" timer — we're back in game
      if (leaveLiveTimerRef.current) {
        clearTimeout(leaveLiveTimerRef.current);
        leaveLiveTimerRef.current = null;
      }
      if (view !== "live") {
        setPrevView(view);
        setView("live");
      }
    } else if (view === "live" && !leaveLiveTimerRef.current) {
      // Debounce leaving live view to avoid flickering during phase transitions
      leaveLiveTimerRef.current = setTimeout(() => {
        leaveLiveTimerRef.current = null;
        setView(prevView);
      }, 1500);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase]);

  useEffect(() => {
    const shouldShowOverlay = overlayEligible && leagueWindowVisible;
    let cancelled = false;

    if (shouldShowOverlay && !overlayShownRef.current) {
      if (overlayRetryTimerRef.current) {
        clearTimeout(overlayRetryTimerRef.current);
        overlayRetryTimerRef.current = null;
      }

      const showOverlays = async () => {
        const [overlayShown, goldOverlayShown] = await Promise.all([
          invoke<boolean>("show_overlay").catch(() => false),
          invoke<boolean>("show_gold_overlay").catch(() => false),
        ]);

        if (cancelled) return;

        overlayShownRef.current = overlayShown || goldOverlayShown;
        if (!overlayShownRef.current && !overlayRetryTimerRef.current) {
          overlayRetryTimerRef.current = setTimeout(() => {
            overlayRetryTimerRef.current = null;
            setOverlayRetryNonce((nonce) => nonce + 1);
          }, 250);
        }
      };

      void showOverlays();
      return () => {
        cancelled = true;
      };
    }

    if (overlayRetryTimerRef.current) {
      clearTimeout(overlayRetryTimerRef.current);
      overlayRetryTimerRef.current = null;
    }

    if (!shouldShowOverlay) {
      overlayShownRef.current = false;
      invoke("hide_overlay").catch(() => {});
      invoke("hide_gold_overlay").catch(() => {});
    }
    return () => {
      cancelled = true;
    };
  }, [leagueWindowVisible, overlayEligible, overlayRetryNonce]);

  useEffect(() => {
    let mounted = true;

    tryDetect();
    invoke<boolean>("get_league_window_visibility")
      .then((visible) => {
        if (mounted) setLeagueWindowVisible(visible);
      })
      .catch(() => {});

    const unlisten = listen<LeagueWindowVisibilityPayload>(
      "league-window-visibility",
      (event) => {
        if (mounted) {
          setLeagueWindowVisible(Boolean(event.payload?.visible));
        }
      }
    );

    return () => {
      mounted = false;
      unlisten.then((dispose) => dispose()).catch(() => {});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (view === "live" || !leaveLiveTimerRef.current) return;
    clearTimeout(leaveLiveTimerRef.current);
    leaveLiveTimerRef.current = null;
  }, [view]);

  useEffect(() => {
    pollRef.current = setInterval(async () => {
      const online = await invoke<boolean>("poll_client_status").catch(
        () => false
      );
      setClientOnline(online);

      if (online && !detectedAccount) {
        tryDetect();
      }
    }, POLL_INTERVAL_MS);

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detectedAccount]);

  useEffect(() => {
    return () => {
      if (overlayRetryTimerRef.current) {
        clearTimeout(overlayRetryTimerRef.current);
        overlayRetryTimerRef.current = null;
      }
      if (leaveLiveTimerRef.current) {
        clearTimeout(leaveLiveTimerRef.current);
        leaveLiveTimerRef.current = null;
      }
    };
  }, []);

  async function tryDetect() {
    try {
      const acc = await invoke<DetectedAccount>("detect_account");
      setDetectedAccount(acc);
      setClientOnline(true);
    } catch {
      const cached = await invoke<DetectedAccount | null>(
        "get_cached_profile"
      ).catch(() => null);
      if (cached) {
        setDetectedAccount(cached);
      }
    }
  }

  async function handleBadgeClick() {
    if (!detectedAccount) return;
    setView("profile");
    await loadDetectedAccount(detectedAccount);
  }

  async function handleSearch(gameName: string, tagLine: string) {
    setView("profile");
    await searchPlayer(gameName, tagLine);
  }

  function goHome() {
    setView("home");
  }

  return (
    <div className="min-h-screen bg-bg-primary">
      <header className="sticky top-0 z-10 bg-bg-primary/80 backdrop-blur-md border-b border-border">
        <div className="max-w-7xl mx-auto px-4 py-4 flex items-center gap-4">
          <button
            onClick={goHome}
            className="flex items-center gap-2 shrink-0 hover:opacity-80 transition-opacity"
          >
            <Eye size={24} className="text-accent" />
            <h1 className="text-lg font-bold text-text-primary">LeagueEye</h1>
          </button>

          {isLive && (
            <button
              onClick={() => setView("live")}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-loss/20 text-loss text-xs font-semibold hover:bg-loss/30 transition-colors"
            >
              <div className="w-1.5 h-1.5 rounded-full bg-loss animate-pulse" />
              LIVE
            </button>
          )}

          <div className="flex-1">
            <SearchBar onSearch={handleSearch} loading={loading} />
          </div>

          {detectedAccount && (
            <AccountBadge
              account={detectedAccount}
              clientOnline={clientOnline}
              onClick={handleBadgeClick}
            />
          )}

          <button
            onClick={() => setView("settings")}
            className="relative p-2 rounded-sm text-text-muted hover:text-text-primary hover:bg-[#1e2130] transition-colors"
            title="Настройки"
          >
            <Settings size={20} />
            {updateAvailable && (
              <div className="absolute top-1 right-1 w-2 h-2 rounded-full bg-[#3b82f6] animate-pulse" />
            )}
          </button>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 py-6">
        {error && (
          <div className="flex items-center gap-3 p-4 rounded-xl bg-loss/10 border border-loss/30 text-loss mb-6">
            <AlertCircle size={20} />
            <span className="text-sm">{error}</span>
          </div>
        )}

        {/* LIVE VIEW */}
        {view === "live" && liveData && liveData.phase !== "none" && (
          <LiveGameView
            data={liveData}
            myPuuid={detectedAccount?.puuid}
          />
        )}

        {/* HOME VIEW */}
        {view === "home" && !loading && !error && (
          <div className="relative">
            <HomeView
              onSearch={handleSearch}
            />
          </div>
        )}

        {/* SETTINGS VIEW */}
        {view === "settings" && (
          <div className="flex flex-col gap-5">
            <button
              onClick={goHome}
              className="flex items-center gap-1 text-text-muted hover:text-text-primary text-sm transition-colors w-fit"
            >
              <ChevronLeft size={16} />
              Назад
            </button>
            <SettingsView />
          </div>
        )}

        {/* LOADING */}
        {loading && !profile && view !== "live" && (
          <div className="flex flex-col items-center justify-center py-24 gap-4">
            <Loader2 size={36} className="animate-spin text-accent" />
            <p className="text-text-muted">Загрузка данных...</p>
          </div>
        )}

        {/* PROFILE VIEW */}
        {view === "profile" && profile && (
          <div className="flex flex-col gap-5">
            <button
              onClick={goHome}
              className="flex items-center gap-1 text-text-muted hover:text-text-primary text-sm transition-colors w-fit"
            >
              <ChevronLeft size={16} />
              Назад
            </button>

            <ProfileCard profile={profile} />

            {loading && (
              <div className="flex items-center gap-2 text-text-muted text-sm">
                <Loader2 size={14} className="animate-spin" />
                Загрузка статистики...
              </div>
            )}

            <MasteryList mastery={mastery} />

            <PlayerTrends
              matches={matches}
              totalCached={totalCached}
              loadMatchesUpTo={loadMatchesUpTo}
            />

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
              <div className="lg:col-span-2">
                <MatchHistory
                  matches={matches}
                  hasMore={hasMore}
                  loadingMore={loadingMore}
                  totalCached={totalCached}
                  totalWins={totalWins}
                  totalLosses={totalLosses}
                  onLoadMore={loadMoreMatches}
                  onPlayerClick={handleSearch}
                />
              </div>
              <div>
                <ChampionStats stats={championStats} />
              </div>
            </div>
          </div>
        )}
      </main>

    </div>
  );
}
