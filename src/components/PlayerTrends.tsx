import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  LineChart,
  Line,
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
  Swords,
  BarChart3,
  Target,
  Shield,
} from "lucide-react";
import type { MatchSummary, MatchupStat } from "../lib/types";
import {
  championIconUrl,
  positionIconUrl,
  positionName,
} from "../lib/ddragon";

// ─── Props ───────────────────────────────────────────────────────────────────

interface Props {
  matches: MatchSummary[];
  puuid: string;
}

// ─── Tabs ────────────────────────────────────────────────────────────────────

type Tab = "trends" | "matchups" | "roles";

const TABS: { key: Tab; label: string; icon: React.ReactNode }[] = [
  { key: "trends", label: "Тренды", icon: <BarChart3 size={14} /> },
  { key: "matchups", label: "Матчапы", icon: <Swords size={14} /> },
  { key: "roles", label: "Роли", icon: <Shield size={14} /> },
];

const GAME_COUNTS = [20, 50, 0] as const; // 0 = all
type GameCount = (typeof GAME_COUNTS)[number];

// ─── Helpers ─────────────────────────────────────────────────────────────────

function computeKda(k: number, d: number, a: number): number {
  return d === 0 ? k + a : (k + a) / d;
}

function computeCsPerMin(cs: number, duration: number): number {
  const mins = duration / 60;
  return mins > 0 ? cs / mins : 0;
}

interface TrendPoint {
  index: number;
  label: string;
  kda: number;
  csPerMin: number;
  visionScore: number;
  win: boolean;
}

function buildTrendData(matches: MatchSummary[]): TrendPoint[] {
  // matches arrive newest-first; reverse so chart reads left=oldest, right=newest
  const chronological = [...matches].reverse();
  return chronological.map((m, i) => ({
    index: i + 1,
    label: `#${i + 1}`,
    kda: Math.round(computeKda(m.kills, m.deaths, m.assists) * 100) / 100,
    csPerMin:
      Math.round(computeCsPerMin(m.cs, m.gameDuration) * 10) / 10,
    visionScore: m.visionScore,
    win: m.win,
  }));
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
      games: 0,
      wins: 0,
      kills: 0,
      deaths: 0,
      assists: 0,
      cs: 0,
      duration: 0,
      vision: 0,
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
      return {
        role,
        games: g,
        wins: s.wins,
        winrate: Math.round((s.wins / g) * 1000) / 10,
        avgKda:
          Math.round(computeKda(s.kills / g, s.deaths / g, s.assists / g) * 100) / 100,
        avgCsPerMin:
          Math.round(computeCsPerMin(s.cs, s.duration) * 10) / 10,
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
  const threshold = avg1 * 0.05; // 5% change threshold
  if (diff > threshold) return "up";
  if (diff < -threshold) return "down";
  return "flat";
}

// ─── Custom tooltip ──────────────────────────────────────────────────────────

function ChartTooltip({
  active,
  payload,
  metric,
}: {
  active?: boolean;
  payload?: Array<{ payload: TrendPoint }>;
  metric: string;
}) {
  if (!active || !payload?.[0]) return null;
  const d = payload[0].payload;
  const value =
    metric === "kda"
      ? d.kda.toFixed(2)
      : metric === "csPerMin"
        ? d.csPerMin.toFixed(1)
        : d.visionScore;
  return (
    <div className="bg-bg-card border border-border rounded-lg px-3 py-2 text-xs shadow-lg">
      <div className="text-text-muted mb-1">
        Игра {d.label}{" "}
        <span className={d.win ? "text-win" : "text-loss"}>
          {d.win ? "W" : "L"}
        </span>
      </div>
      <div className="text-text-primary font-medium">
        {metric === "kda" ? "KDA" : metric === "csPerMin" ? "CS/min" : "Vision"}: {value}
      </div>
    </div>
  );
}

// ─── Main Component ──────────────────────────────────────────────────────────

export function PlayerTrends({ matches, puuid }: Props) {
  const [tab, setTab] = useState<Tab>("trends");
  const [gameCount, setGameCount] = useState<GameCount>(20);
  const [matchups, setMatchups] = useState<MatchupStat[]>([]);
  const [matchupsLoading, setMatchupsLoading] = useState(false);
  const [matchupsLoaded, setMatchupsLoaded] = useState(false);

  // Load matchups when tab switches
  useEffect(() => {
    if (tab !== "matchups" || matchupsLoaded || matchupsLoading) return;
    setMatchupsLoading(true);
    invoke<MatchupStat[]>("get_matchups", { puuid })
      .then((data) => {
        setMatchups(data);
        setMatchupsLoaded(true);
      })
      .catch(() => setMatchups([]))
      .finally(() => setMatchupsLoading(false));
  }, [tab, puuid, matchupsLoaded, matchupsLoading]);

  // Reset matchups when puuid changes
  useEffect(() => {
    setMatchups([]);
    setMatchupsLoaded(false);
  }, [puuid]);

  const slicedMatches = useMemo(() => {
    if (gameCount === 0) return matches;
    return matches.slice(0, gameCount);
  }, [matches, gameCount]);

  const trendData = useMemo(
    () => buildTrendData(slicedMatches),
    [slicedMatches]
  );

  const roleStats = useMemo(() => buildRoleStats(matches), [matches]);

  const kdaTrend = useMemo(
    () => trendIndicator(trendData.map((d) => d.kda)),
    [trendData]
  );
  const csTrend = useMemo(
    () => trendIndicator(trendData.map((d) => d.csPerMin)),
    [trendData]
  );
  const visionTrend = useMemo(
    () => trendIndicator(trendData.map((d) => d.visionScore)),
    [trendData]
  );

  if (matches.length < 3) return null;

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
          <div className="flex gap-1">
            {GAME_COUNTS.map((c) => (
              <button
                key={c}
                onClick={() => setGameCount(c)}
                className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                  gameCount === c
                    ? "bg-accent/20 text-accent"
                    : "bg-bg-secondary text-text-muted hover:text-text-primary"
                }`}
              >
                {c === 0 ? `Все (${matches.length})` : `${c} игр`}
              </button>
            ))}
          </div>

          {/* KDA Chart */}
          <TrendChart
            title="KDA"
            data={trendData}
            dataKey="kda"
            color="#3b82f6"
            trend={kdaTrend}
            format={(v) => v.toFixed(2)}
            metric="kda"
          />

          {/* CS/min Chart */}
          <TrendChart
            title="CS/min"
            data={trendData}
            dataKey="csPerMin"
            color="#eab308"
            trend={csTrend}
            format={(v) => v.toFixed(1)}
            metric="csPerMin"
          />

          {/* Vision Score Chart */}
          <TrendChart
            title="Vision Score"
            data={trendData}
            dataKey="visionScore"
            color="#22c55e"
            trend={visionTrend}
            format={(v) => Math.round(v).toString()}
            metric="visionScore"
          />
        </div>
      )}

      {/* Matchups Tab */}
      {tab === "matchups" && (
        <MatchupsView
          matchups={matchups}
          loading={matchupsLoading}
        />
      )}

      {/* Roles Tab */}
      {tab === "roles" && <RolesView roleStats={roleStats} />}
    </div>
  );
}

// ─── TrendChart ──────────────────────────────────────────────────────────────

function TrendChart({
  title,
  data,
  dataKey,
  color,
  trend,
  format,
  metric,
}: {
  title: string;
  data: TrendPoint[];
  dataKey: keyof TrendPoint;
  color: string;
  trend: "up" | "down" | "flat";
  format: (v: number) => string;
  metric: string;
}) {
  const values = data.map((d) => d[dataKey] as number);
  const avg = values.length
    ? values.reduce((a, b) => a + b, 0) / values.length
    : 0;
  const current = values.length ? values[values.length - 1] : 0;

  return (
    <div className="bg-bg-secondary rounded-lg p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-text-primary">{title}</span>
          <TrendBadge trend={trend} />
        </div>
        <div className="flex items-center gap-3 text-xs text-text-muted">
          <span>Текущее: <span className="text-text-primary font-medium">{format(current)}</span></span>
          <span>Среднее: <span className="text-text-primary font-medium">{format(avg)}</span></span>
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
              tickFormatter={(v) => format(v)}
            />
            <Tooltip content={<ChartTooltip metric={metric} />} />
            <ReferenceLine y={avg} stroke="#64748b" strokeDasharray="3 3" />
            <Line
              type="monotone"
              dataKey={dataKey}
              stroke={color}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, fill: color }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

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

// ─── MatchupsView ────────────────────────────────────────────────────────────

function MatchupsView({
  matchups,
  loading,
}: {
  matchups: MatchupStat[];
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-8 gap-2 text-text-muted">
        <Loader2 size={16} className="animate-spin" />
        <span className="text-sm">Загрузка матчапов...</span>
      </div>
    );
  }

  if (matchups.length === 0) {
    return (
      <div className="text-center py-8 text-text-muted text-sm">
        Нет данных о матчапах. Нужно минимум 2 игры против одного чемпиона на той же позиции.
      </div>
    );
  }

  // Split into best (wr >= 50, sorted by wr desc) and worst (wr < 50, sorted by wr asc)
  const best = matchups
    .filter((m) => m.winrate >= 50)
    .sort((a, b) => b.winrate - a.winrate || b.games - a.games)
    .slice(0, 10);

  const worst = matchups
    .filter((m) => m.winrate < 50)
    .sort((a, b) => a.winrate - b.winrate || b.games - a.games)
    .slice(0, 10);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <MatchupTable
        title="Лучшие матчапы"
        icon={<Target size={14} className="text-win" />}
        matchups={best}
        emptyText="Нет выигрышных матчапов"
      />
      <MatchupTable
        title="Худшие матчапы"
        icon={<Swords size={14} className="text-loss" />}
        matchups={worst}
        emptyText="Нет проигрышных матчапов"
      />
    </div>
  );
}

function MatchupTable({
  title,
  icon,
  matchups,
  emptyText,
}: {
  title: string;
  icon: React.ReactNode;
  matchups: MatchupStat[];
  emptyText: string;
}) {
  return (
    <div className="bg-bg-secondary rounded-lg p-3">
      <div className="flex items-center gap-2 mb-2">
        {icon}
        <span className="text-xs font-semibold text-text-primary">{title}</span>
      </div>
      {matchups.length === 0 ? (
        <div className="text-center py-4 text-text-muted text-xs">{emptyText}</div>
      ) : (
        <table className="w-full text-xs">
          <thead>
            <tr className="text-text-muted uppercase">
              <th className="text-left py-1 px-1">Противник</th>
              <th className="text-center py-1 px-1">Поз.</th>
              <th className="text-center py-1 px-1">Игр</th>
              <th className="text-center py-1 px-1">WR</th>
              <th className="text-center py-1 px-1">KDA</th>
            </tr>
          </thead>
          <tbody>
            {matchups.map((m) => {
              const kda = computeKda(m.avgKills, m.avgDeaths, m.avgAssists);
              return (
                <tr
                  key={`${m.enemyChampionName}-${m.position}`}
                  className="border-t border-border/50 hover:bg-bg-hover transition-colors"
                >
                  <td className="py-1.5 px-1">
                    <div className="flex items-center gap-1.5">
                      <img
                        src={championIconUrl(m.enemyChampionName)}
                        alt={m.enemyChampionName}
                        className="w-6 h-6 rounded"
                      />
                      <span className="text-text-primary font-medium">
                        {m.enemyChampionName}
                      </span>
                    </div>
                  </td>
                  <td className="text-center py-1.5 px-1">
                    <img
                      src={positionIconUrl(m.position)}
                      alt={m.position}
                      className="w-3.5 h-3.5 mx-auto opacity-50"
                    />
                  </td>
                  <td className="text-center py-1.5 px-1 text-text-secondary">
                    {m.games}
                  </td>
                  <td className="text-center py-1.5 px-1">
                    <span
                      className={`font-semibold ${
                        m.winrate >= 60
                          ? "text-win"
                          : m.winrate >= 50
                            ? "text-text-primary"
                            : "text-loss"
                      }`}
                    >
                      {m.winrate}%
                    </span>
                  </td>
                  <td className="text-center py-1.5 px-1">
                    <span
                      className={`font-medium ${
                        kda >= 4
                          ? "text-gold"
                          : kda >= 3
                            ? "text-win"
                            : kda >= 2
                              ? "text-text-primary"
                              : "text-loss"
                      }`}
                    >
                      {kda.toFixed(1)}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
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
