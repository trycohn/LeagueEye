import type { MatchDetail, MatchParticipantDetail } from "../lib/types";
import {
  championIconUrl,
  itemIconUrl,
  summonerSpellIconUrl,
} from "../lib/ddragon";

interface Props {
  detail: MatchDetail;
  onPlayerClick: (gameName: string, tagLine: string) => void;
}

const POSITION_LABEL: Record<string, string> = {
  TOP: "Top",
  JUNGLE: "Jungle",
  MIDDLE: "Mid",
  BOTTOM: "ADC",
  UTILITY: "Support",
};

const GRID_COLS =
  "grid-cols-[minmax(115px,1fr)_62px_48px_36px_50px_minmax(0,1fr)_minmax(168px,max-content)]";

function formatGold(gold: number): string {
  if (gold >= 1000) return `${(gold / 1000).toFixed(1)}k`;
  return gold.toString();
}

export function MatchDetailView({ detail, onPlayerClick }: Props) {
  const team100 = detail.participants.filter((p) => p.teamId === 100);
  const team200 = detail.participants.filter((p) => p.teamId === 200);

  const maxDamage = Math.max(...detail.participants.map((p) => p.damage), 1);

  const team100Kills = team100.reduce((s, p) => s + p.kills, 0);
  const team200Kills = team200.reduce((s, p) => s + p.kills, 0);

  const team100Win = team100[0]?.win ?? false;
  const team200Win = team200[0]?.win ?? false;

  const gameDurationMin = detail.gameDuration / 60;

  return (
    <div className="p-3 flex flex-col gap-3 bg-bg-primary/50 overflow-x-auto">
      <TeamTable
        label={team100Win ? "СИНЯЯ КОМАНДА — ПОБЕДА" : "СИНЯЯ КОМАНДА — ПОРАЖЕНИЕ"}
        win={team100Win}
        players={team100}
        teamKills={team100Kills}
        maxDamage={maxDamage}
        gameDurationMin={gameDurationMin}
        onPlayerClick={onPlayerClick}
      />
      <TeamTable
        label={team200Win ? "КРАСНАЯ КОМАНДА — ПОБЕДА" : "КРАСНАЯ КОМАНДА — ПОРАЖЕНИЕ"}
        win={team200Win}
        players={team200}
        teamKills={team200Kills}
        maxDamage={maxDamage}
        gameDurationMin={gameDurationMin}
        onPlayerClick={onPlayerClick}
      />
    </div>
  );
}

interface TeamTableProps {
  label: string;
  win: boolean;
  players: MatchParticipantDetail[];
  teamKills: number;
  maxDamage: number;
  gameDurationMin: number;
  onPlayerClick: (gameName: string, tagLine: string) => void;
}

function TeamTable({
  label,
  win,
  players,
  teamKills,
  maxDamage,
  gameDurationMin,
  onPlayerClick,
}: TeamTableProps) {
  const totalKills = players.reduce((s, p) => s + p.kills, 0);
  const totalDeaths = players.reduce((s, p) => s + p.deaths, 0);
  const totalAssists = players.reduce((s, p) => s + p.assists, 0);
  const totalGold = players.reduce((s, p) => s + p.gold, 0);

  return (
    <div className="rounded-lg overflow-hidden border border-border/50 min-w-0">
      {/* Team header */}
      <div
        className={`flex items-center justify-between px-3 py-2 text-xs font-semibold uppercase tracking-wider ${
          win
            ? "bg-win/10 text-win border-b border-win/20"
            : "bg-loss/10 text-loss border-b border-loss/20"
        }`}
      >
        <span>{label}</span>
        <div className="flex items-center gap-3 text-text-secondary font-normal normal-case">
          <span>
            {totalKills}/{totalDeaths}/{totalAssists}
          </span>
          <span>{formatGold(totalGold)} золота</span>
        </div>
      </div>

      {/* Column headers */}
      <div className={`grid ${GRID_COLS} gap-x-2 gap-y-1 px-3 py-1.5 bg-bg-secondary/50 text-[10px] text-text-muted uppercase tracking-wider items-center`}>
        <span>Игрок</span>
        <span className="text-center">KDA</span>
        <span className="text-center">CS</span>
        <span className="text-center">КП%</span>
        <span className="text-center">Золото</span>
        <span className="text-center">Урон</span>
        <span className="text-center">Предметы</span>
      </div>

      {/* Player rows */}
      {players.map((p) => (
        <PlayerRow
          key={p.puuid}
          player={p}
          teamKills={teamKills}
          maxDamage={maxDamage}
          gameDurationMin={gameDurationMin}
          onPlayerClick={onPlayerClick}
        />
      ))}
    </div>
  );
}

interface PlayerRowProps {
  player: MatchParticipantDetail;
  teamKills: number;
  maxDamage: number;
  gameDurationMin: number;
  onPlayerClick: (gameName: string, tagLine: string) => void;
}

function PlayerRow({
  player: p,
  teamKills,
  maxDamage,
  gameDurationMin,
  onPlayerClick,
}: PlayerRowProps) {
  const kp = teamKills > 0 ? Math.round(((p.kills + p.assists) / teamKills) * 100) : 0;
  const csPerMin = gameDurationMin > 0 ? (p.cs / gameDurationMin).toFixed(1) : "0";
  const damagePercent = maxDamage > 0 ? (p.damage / maxDamage) * 100 : 0;

  const kda =
    p.deaths === 0
      ? "Perfect"
      : ((p.kills + p.assists) / p.deaths).toFixed(1);

  const displayName = p.riotIdName || "Unknown";
  const canClick = p.riotIdName && p.riotIdTagline;

  return (
    <div className={`grid ${GRID_COLS} gap-x-2 items-center px-3 py-1.5 border-t border-border/30 hover:bg-bg-secondary/30 transition-colors min-w-0`}>
      {/* Player info */}
      <div className="flex items-center gap-1.5 min-w-0 overflow-hidden">
        <div className="relative shrink-0">
          <img
            src={championIconUrl(p.championName)}
            alt={p.championName}
            className="w-8 h-8 rounded"
          />
          <span className="absolute -bottom-0.5 -right-0.5 bg-bg-primary text-[9px] text-text-muted rounded px-0.5 leading-tight border border-border/50">
            {p.champLevel}
          </span>
        </div>
        <div className="flex flex-col gap-0.5 shrink-0">
          {p.summonerSpells.map((spellId, i) => (
            <img
              key={i}
              src={summonerSpellIconUrl(spellId)}
              alt=""
              className="w-3.5 h-3.5 rounded-sm"
            />
          ))}
        </div>
        <div className="min-w-0 flex-1">
          {canClick ? (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onPlayerClick(p.riotIdName, p.riotIdTagline);
              }}
              className="text-xs text-text-primary hover:text-accent transition-colors truncate block w-full text-left"
              title={`${p.riotIdName}#${p.riotIdTagline}`}
            >
              {displayName}
            </button>
          ) : (
            <span className="text-xs text-text-muted truncate block">{displayName}</span>
          )}
          {p.position && POSITION_LABEL[p.position.toUpperCase()] && (
            <span className="text-[9px] text-text-muted">
              {POSITION_LABEL[p.position.toUpperCase()]}
            </span>
          )}
        </div>
      </div>

      {/* KDA */}
      <div className="text-center min-w-0">
        <div className="text-xs text-text-primary">
          {p.kills}/{p.deaths}/{p.assists}
        </div>
        <div className="text-[9px] text-text-muted">
          <span
            className={
              kda === "Perfect"
                ? "text-gold"
                : parseFloat(kda) >= 3
                  ? "text-win"
                  : parseFloat(kda) >= 2
                    ? "text-text-secondary"
                    : "text-loss"
            }
          >
            {kda}
          </span>
        </div>
      </div>

      {/* CS */}
      <div className="text-center min-w-0">
        <div className="text-xs text-text-primary">{p.cs}</div>
        <div className="text-[9px] text-text-muted">{csPerMin}/m</div>
      </div>

      {/* Kill Participation */}
      <div className="text-center min-w-0">
        <span
          className={`text-xs ${
            kp >= 60 ? "text-win" : kp >= 40 ? "text-text-primary" : "text-text-muted"
          }`}
        >
          {kp}%
        </span>
      </div>

      {/* Gold */}
      <div className="text-center min-w-0">
        <div className="text-xs text-gold">{formatGold(p.gold)}</div>
        <div className="text-[9px] text-text-muted">{p.gold.toLocaleString()}</div>
      </div>

      {/* Damage bar — min-w-0 + overflow so bar never bleeds into items column */}
      <div className="flex flex-col gap-0.5 min-w-0 max-w-full pr-1">
        <div className="h-2.5 w-full max-w-full bg-bg-secondary rounded-full overflow-hidden">
          <div
            className="h-full max-w-full bg-loss/70 rounded-full"
            style={{ width: `${damagePercent}%` }}
          />
        </div>
        <div className="text-[9px] text-text-muted text-center tabular-nums truncate">
          {p.damage.toLocaleString()}
        </div>
      </div>

      {/* Items — reserved width, never shrink */}
      <div className="flex gap-0.5 justify-end shrink-0 pl-1">
        {p.items.map((itemId, i) =>
          itemId > 0 ? (
            <img
              key={i}
              src={itemIconUrl(itemId)}
              alt=""
              className="w-5 h-5 rounded-sm"
            />
          ) : (
            <div
              key={i}
              className="w-5 h-5 rounded-sm bg-bg-secondary border border-border/30"
            />
          )
        )}
      </div>
    </div>
  );
}
