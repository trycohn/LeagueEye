import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SearchBar } from "./components/SearchBar";
import { ProfileCard } from "./components/ProfileCard";
import { MasteryList } from "./components/MasteryList";
import { MatchHistory } from "./components/MatchHistory";
import { ChampionStats } from "./components/ChampionStats";
import { AccountBadge } from "./components/AccountBadge";
import { LiveGameView } from "./components/LiveGameView";
import { useRiotApi } from "./hooks/useRiotApi";
import { useLiveGame } from "./hooks/useLiveGame";
import { HomeView } from "./components/HomeView";
import { Eye, AlertCircle, Loader2, ChevronLeft, FlaskConical } from "lucide-react";
import type { DetectedAccount } from "./lib/types";
import { AiTestDialog } from "./components/AiTestDialog"; // TEST: удалить после тестирования

type View = "home" | "profile" | "live";

const POLL_INTERVAL_MS = 8_000;

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
  } = useRiotApi();

  const [view, setView] = useState<View>("home");
  const [prevView, setPrevView] = useState<View>("home");
  const [detectedAccount, setDetectedAccount] =
    useState<DetectedAccount | null>(null);
  const [clientOnline, setClientOnline] = useState(false);
  const [showAiTest, setShowAiTest] = useState(false); // TEST: удалить после тестирования
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const { liveData, phase } = useLiveGame(clientOnline);

  // Автопереключение на live view + автопоказ оверлея
  const overlayShownRef = useRef(false);
  const leaveLiveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (phase === "champ_select" || phase === "in_game") {
      // Cancel any pending "leave live" timer — we're back in game
      if (leaveLiveTimerRef.current) {
        clearTimeout(leaveLiveTimerRef.current);
        leaveLiveTimerRef.current = null;
      }
      if (view !== "live") {
        setPrevView(view);
        setView("live");
      }
      if (!overlayShownRef.current) {
        overlayShownRef.current = true;
        invoke("show_overlay").catch(() => {});
        invoke("show_gold_overlay").catch(() => {});
      }
    } else if (view === "live" && !leaveLiveTimerRef.current) {
      // Debounce leaving live view to avoid flickering during phase transitions
      leaveLiveTimerRef.current = setTimeout(() => {
        leaveLiveTimerRef.current = null;
        setView(prevView);
        overlayShownRef.current = false;
        invoke("hide_overlay").catch(() => {});
        invoke("hide_gold_overlay").catch(() => {});
      }, 1500);
    }
    return () => {
      if (leaveLiveTimerRef.current) {
        clearTimeout(leaveLiveTimerRef.current);
        leaveLiveTimerRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase]);

  useEffect(() => {
    tryDetect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  const isLive = phase === "champ_select" || phase === "in_game";

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

          {/* TEST: кнопка тестирования AI, удалить после тестирования */}
          <button
            onClick={() => setShowAiTest(true)}
            className="flex items-center gap-1 px-2 py-1 rounded-lg bg-loss/10 text-loss text-xs font-medium hover:bg-loss/20 transition-colors border border-loss/20"
            title="Тест AI стриминга"
          >
            <FlaskConical size={14} />
            AI Test
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

      {/* TEST: диалог тестирования AI, удалить после тестирования */}
      {showAiTest && <AiTestDialog onClose={() => setShowAiTest(false)} />}
    </div>
  );
}
