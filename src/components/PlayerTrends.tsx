import { useCallback, useMemo, useRef, useState } from "react";
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  ReferenceLine,
} from "recharts";
import {
  TrendingUp,
  TrendingDown,
  Minus,
  Loader2,
  BarChart3,
  Shield,
} from "lucide-react";
import type { MatchSummary } from "../lib/types";
import { positionIconUrl, positionName } from "../lib/ddragon";

// ─── Props ───────────────────────────────────────────────────────────────────

interface Props {
  matches: MatchSummary[];
  totalCached: number;
  hasMore: boolean;
  loadMoreMatches: () => Promise<void>;
}

// ─── Tabs ────────────────────────────────────────────────────────────────────

type Tab = "trends" | "roles";

const TABS: { key: Tab; label: string; icon: React.ReactNode }[] = [
  { key: "trends", label: "Тренды", icon: <BarChart3 size={14} /> },
  { key: "roles", label: "Роли", icon: <Shield size={14} /> },
];

// ─── Helpers ─────────────────────────────────────────────────────────────────

function computeKda(k: number, d: number, a: number): number {
  return d === 0 ? k + a : (k + a) / d;
}

interface TrendPoint {
  index: number;
  label: string;
  kda: number;
  win: boolean;
}

function buildTrendData(matches: MatchSummary[]): TrendPoint[] {
  const chronological = [...matches].reverse();
  return chronological.map((m, i) => ({
    index: i + 1,
    label: `#${i + 1}`,
    kda: Math.round(computeKda(m.kills, m.deaths, m.assists) * 100) / 100,
    win: m.win,
  }));
}

interface WinratePoint {
  index: number;
  label: string;
  winrate: number;
  win: boolean;
}

function buildWinrateData(matches: MatchSummary[]): WinratePoint[] {
  const chronological = [...matches].reverse();
  let wins = 0;
  return chronological.map((m, i) => {
    if (m.win) wins++;
    const total = i + 1;
    return {
      index: total,
      label: `#${total}`,
      winrate: Math.round((wins / total) * 1000) / 10,
      win: m.win,
    };
  });
}

interface RoleStat {
  role: string;
  games: number;
  wins: number;
  winrate: number;
  avgKda: number;
  avgCsPerMin: number;
  avgVisionScore: number;
}

function buildRoleStats(matches: MatchSummary[]): RoleStat[] {
  const map = new Map<
    string,
    {
      games: number;
      wins: number;
      kills: number;
      deaths: number;
      assists: number;
      cs: number;
      duration: number;
      vision: number;
    }
  >();
  for (const m of matches) {
    const pos = m.position.toUpperCase();
    if (!pos || pos === "UNKNOWN") continue;
    const stat = map.get(pos) || {
      games: 0, wins: 0, kills: 0, deaths: 0, assists: 0,
      cs: 0, duration: 0, vision: 0,
    };
    stat.games++;
    if (m.win) stat.wins++;
    stat.kills += m.kills;
    stat.deaths += m.deaths;
    stat.assists += m.assists;
    stat.cs += m.cs;
    stat.duration += m.gameDuration;
    stat.vision += m.visionScore;
    map.set(pos, stat);
  }

  const order = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];
  return order
    .filter((r) => map.has(r))
    .map((role) => {
      const s = map.get(role)!;
      const g = s.games;
      const mins = s.duration / 60;
      return {
        role,
        games: g,
        wins: s.wins,
        winrate: Math.round((s.wins / g) * 1000) / 10,
        avgKda: Math.round(computeKda(s.kills / g, s.deaths / g, s.assists / g) * 100) / 100,
        avgCsPerMin: mins > 0 ? Math.round((s.cs / mins) * 10) / 10 : 0,
        avgVisionScore: Math.round((s.vision / g) * 10) / 10,
      };
    });
}

function trendIndicator(data: number[]): "up" | "down" | "flat" {
  if (data.length < 4) return "flat";
  const half = Math.floor(data.length / 2);
  const firstHalf = data.slice(0, half);
  const secondHalf = data.slice(half);
  const avg1 = firstHalf.reduce((a, b) => a + b, 0) / firstHalf.length;
  const avg2 = secondHalf.reduce((a, b) => a + b, 0) / secondHalf.length;
  const diff = avg2 - avg1;
  const threshold = avg1 * 0.05;
  if (diff > threshold) return "up";
  if (diff < -threshold) return "down";
  return "flat";
}

// ─── Custom tooltips ─────────────────────────────────────────────────────────

function KdaTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: Array<{ payload: TrendPoint }>;
}) {
  if (!active || !payload?.[0]) return null;
  const d = payload[0].payload;
  return (
    <div className="bg-bg-card border border-border rounded-lg px-3 py-2 text-xs shadow-lg">
      <div className="text-text-muted mb-1">
        Игра {d.label}{" "}
        <span className={d.win ? "text-win" : "text-loss"}>
          {d.win ? "W" : "L"}
        </span>
      </div>
      <div className="text-text-primary font-medium">KDA: {d.kda.toFixed(2)}</div>
    </div>
  );
}

function WinrateTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: Array<{ payload: WinratePoint }>;
}) {
  if (!active || !payload?.[0]) return null;
  const d = payload[0].payload;
  return (
    <div className="bg-bg-card border border-border rounded-lg px-3 py-2 text-xs shadow-lg">
      <div className="text-text-muted mb-1">
        Игра {d.label}{" "}
        <span className={d.win ? "text-win" : "text-loss"}>
          {d.win ? "W" : "L"}
        </span>
      </div>
      <div className="text-text-primary font-medium">Winrate: {d.winrate}%</div>
    </div>
  );
}

// ─── Game count options ──────────────────────────────────────────────────────

type GameCount = 20 | 50 | 0; // 0 = all

const PAGE_SIZE = 15;

// ─── Main Component ──────────────────────────────────────────────────────────

export function PlayerTrends({
  matches,
  totalCached,
  hasMore,
  loadMoreMatches,
}: Props) {
  const [tab, setTab] = useState<Tab>("trends");
  const [gameCount, setGameCount] = useState<GameCount>(20);
  const [loadingExtra, setLoadingExtra] = useState(false);
  const loadingRef = useRef(false);

  // Load enough matches for the requested game count
  const ensureMatches = useCallback(
    async (target: number) => {
      // target=0 means load ALL
      if (loadingRef.current) return;

      const needed = target === 0 ? totalCached : target;
      if (matches.length >= needed || !hasMore) return;

      loadingRef.current = true;
      setLoadingExtra(true);
      try {
        // Keep calling loadMoreMatches until we have enough
        let safety = 0;
        const maxIterations = Math.ceil((needed - matches.length) / PAGE_SIZE) + 1;
        while (safety < maxIterations) {
          safety++;
          await loadMoreMatches();
          // Re-check after each load — we read from the hook's state via closure,
          // but since loadMoreMatches mutates state, we need to break based on
          // how many pages we've requested
          const loadedSoFar = matches.length + safety * PAGE_SIZE;
          if (loadedSoFar >= needed) break;
        }
      } finally {
        loadingRef.current = false;
        setLoadingExtra(false);
      }
    },
    [matches.length, totalCached, hasMore, loadMoreMatches]
  );

  const handleGameCount = useCallback(
    (count: GameCount) => {
      setGameCount(count);
      if (count === 0) {
        void ensureMatches(0);
      } else if (matches.length < count && hasMore) {
        void ensureMatches(count);
      }
    },
    [ensureMatches, matches.length, hasMore]
  );

  const slicedMatches = useMemo(() => {
    if (gameCount === 0) return matches;
    return matches.slice(0, gameCount);
  }, [matches, gameCount]);

  const trendData = useMemo(() => buildTrendData(slicedMatches), [slicedMatches]);
  const winrateData = useMemo(() => buildWinrateData(slicedMatches), [slicedMatches]);
  const roleStats = useMemo(() => buildRoleStats(matches), [matches]);

  const kdaTrend = useMemo(
    () => trendIndicator(trendData.map((d) => d.kda)),
    [trendData]
  );
  const wrTrend = useMemo(
    () => trendIndicator(winrateData.map((d) => d.winrate)),
    [winrateData]
  );

  if (matches.length < 3) return null;

  // Build visible game count options, hide 50 if totalCached < 50
  const gameCountOptions: { value: GameCount; label: string }[] = [
    { value: 20, label: "20 игр" },
  ];
  if (totalCached >= 50) {
    gameCountOptions.push({ value: 50, label: "50 игр" });
  }
  if (totalCached > 20) {
    gameCountOptions.push({ value: 0, label: `Все (${totalCached})` });
  }

  return (
    <div className="rounded-xl bg-bg-card border border-border p-4">
      {/* Header with tabs */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wider">
          Аналитика
        </h3>
        <div className="flex gap-1">
          {TABS.map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded text-xs font-medium transition-colors ${
                tab === t.key
                  ? "bg-accent text-white"
                  : "bg-bg-secondary text-text-muted hover:text-text-primary"
              }`}
            >
              {t.icon}
              {t.label}
            </button>
          ))}
        </div>
      </div>

      {/* Trends Tab */}
      {tab === "trends" && (
        <div className="space-y-4">
          {/* Game count toggle */}
          <div className="flex items-center gap-1">
            {gameCountOptions.map((opt) => (
              <button
                key={opt.value}
                onClick={() => handleGameCount(opt.value)}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  gameCount === opt.value
                    ? "bg-accent/20 text-accent"
                    : "bg-bg-secondary text-text-muted hover:text-text-primary"
                }`}
              >
                {opt.label}
              </button>
            ))}
            {loadingExtra && (
              <div className="flex items-center gap-1.5 text-text-muted text-xs ml-2">
                <Loader2 size={12} className="animate-spin" />
                Загрузка...
              </div>
            )}
          </div>

          {/* KDA Chart */}
          <KdaChart data={trendData} trend={kdaTrend} />

          {/* Winrate Chart */}
          <WinrateChart data={winrateData} trend={wrTrend} />
        </div>
      )}

      {/* Roles Tab */}
      {tab === "roles" && <RolesView roleStats={roleStats} />}
    </div>
  );
}

// ─── KDA Chart ───────────────────────────────────────────────────────────────

function KdaChart({
  data,
  trend,
}: {
  data: TrendPoint[];
  trend: "up" | "down" | "flat";
}) {
  const values = data.map((d) => d.kda);
  const avg = values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0;
  const current = values.length ? values[values.length - 1] : 0;

  return (
    <div className="bg-bg-secondary rounded-lg p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-text-primary">KDA</span>
          <TrendBadge trend={trend} />
        </div>
        <div className="flex items-center gap-3 text-xs text-text-muted">
          <span>
            Текущее:{" "}
            <span className="text-text-primary font-medium">{current.toFixed(2)}</span>
          </span>
          <span>
            Среднее:{" "}
            <span className="text-text-primary font-medium">{avg.toFixed(2)}</span>
          </span>
        </div>
      </div>
      <div className="h-32">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#2a2d3a" />
            <XAxis
              dataKey="index"
              tick={{ fontSize: 10, fill: "#64748b" }}
              tickLine={false}
              axisLine={{ stroke: "#2a2d3a" }}
              interval={Math.max(0, Math.floor(data.length / 8) - 1)}
            />
            <YAxis
              tick={{ fontSize: 10, fill: "#64748b" }}
              tickLine={false}
              axisLine={false}
              width={35}
              tickFormatter={(v) => v.toFixed(1)}
            />
            <Tooltip content={<KdaTooltip />} />
            <ReferenceLine y={avg} stroke="#64748b" strokeDasharray="3 3" />
            <Line
              type="monotone"
              dataKey="kda"
              stroke="#3b82f6"
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, fill: "#3b82f6" }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ─── Winrate Chart ───────────────────────────────────────────────────────────

function WinrateChart({
  data,
  trend,
}: {
  data: WinratePoint[];
  trend: "up" | "down" | "flat";
}) {
  const current = data.length ? data[data.length - 1].winrate : 0;
  const totalWins = data.filter((d) => d.win).length;
  const totalLosses = data.length - totalWins;

  return (
    <div className="bg-bg-secondary rounded-lg p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-text-primary">Winrate</span>
          <TrendBadge trend={trend} />
        </div>
        <div className="flex items-center gap-3 text-xs text-text-muted">
          <span>
            Текущий:{" "}
            <span
              className={`font-medium ${
                current >= 50 ? "text-win" : "text-loss"
              }`}
            >
              {current}%
            </span>
          </span>
          <span>
            <span className="text-win">{totalWins}W</span>
            {" / "}
            <span className="text-loss">{totalLosses}L</span>
          </span>
        </div>
      </div>
      <div className="h-32">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data}>
            <defs>
              <linearGradient id="winrateGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#22c55e" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#22c55e" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#2a2d3a" />
            <XAxis
              dataKey="index"
              tick={{ fontSize: 10, fill: "#64748b" }}
              tickLine={false}
              axisLine={{ stroke: "#2a2d3a" }}
              interval={Math.max(0, Math.floor(data.length / 8) - 1)}
            />
            <YAxis
              tick={{ fontSize: 10, fill: "#64748b" }}
              tickLine={false}
              axisLine={false}
              width={35}
              domain={[0, 100]}
              tickFormatter={(v) => `${v}%`}
            />
            <Tooltip content={<WinrateTooltip />} />
            <ReferenceLine y={50} stroke="#ef4444" strokeDasharray="3 3" strokeOpacity={0.5} />
            <Area
              type="monotone"
              dataKey="winrate"
              stroke="#22c55e"
              strokeWidth={2}
              fill="url(#winrateGrad)"
              dot={false}
              activeDot={{ r: 4, fill: "#22c55e" }}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ─── TrendBadge ──────────────────────────────────────────────────────────────

function TrendBadge({ trend }: { trend: "up" | "down" | "flat" }) {
  if (trend === "up")
    return (
      <span className="flex items-center gap-0.5 text-win text-xs">
        <TrendingUp size={12} />
      </span>
    );
  if (trend === "down")
    return (
      <span className="flex items-center gap-0.5 text-loss text-xs">
        <TrendingDown size={12} />
      </span>
    );
  return (
    <span className="flex items-center gap-0.5 text-text-muted text-xs">
      <Minus size={12} />
    </span>
  );
}

// ─── RolesView ───────────────────────────────────────────────────────────────

function RolesView({ roleStats }: { roleStats: RoleStat[] }) {
  if (roleStats.length === 0) {
    return (
      <div className="text-center py-8 text-text-muted text-sm">
        Нет данных по ролям
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
      {roleStats.map((r) => (
        <div
          key={r.role}
          className="bg-bg-secondary rounded-lg p-3 flex flex-col gap-2"
        >
          <div className="flex items-center gap-2">
            <img
              src={positionIconUrl(r.role)}
              alt={r.role}
              className="w-5 h-5"
              style={{ filter: "brightness(0.8)" }}
            />
            <span className="text-sm font-semibold text-text-primary">
              {positionName(r.role)}
            </span>
            <span className="text-xs text-text-muted ml-auto">
              {r.games} {r.games === 1 ? "игра" : r.games < 5 ? "игры" : "игр"}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
            <div className="flex justify-between">
              <span className="text-text-muted">Winrate</span>
              <span
                className={`font-semibold ${
                  r.winrate >= 60
                    ? "text-win"
                    : r.winrate >= 50
                      ? "text-text-primary"
                      : "text-loss"
                }`}
              >
                {r.winrate}%
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">KDA</span>
              <span
                className={`font-medium ${
                  r.avgKda >= 4
                    ? "text-gold"
                    : r.avgKda >= 3
                      ? "text-win"
                      : r.avgKda >= 2
                        ? "text-text-primary"
                        : "text-loss"
                }`}
              >
                {r.avgKda.toFixed(2)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">CS/min</span>
              <span className="text-text-primary font-medium">
                {r.avgCsPerMin}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Vision</span>
              <span className="text-text-primary font-medium">
                {r.avgVisionScore}
              </span>
            </div>
          </div>

          {/* Win/Loss bar */}
          <div className="flex h-1.5 rounded-full overflow-hidden bg-bg-primary mt-1">
            <div
              className="bg-win rounded-l-full"
              style={{ width: `${r.winrate}%` }}
            />
            <div
              className="bg-loss rounded-r-full"
              style={{ width: `${100 - r.winrate}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-text-muted">
            <span className="text-win">{r.wins}W</span>
            <span className="text-loss">{r.games - r.wins}L</span>
          </div>
        </div>
      ))}
    </div>
  );
}
