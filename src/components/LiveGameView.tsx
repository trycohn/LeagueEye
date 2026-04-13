import type { LiveGameData } from "../lib/types";
import { LivePlayerRow } from "./LivePlayerRow";
import { CoachPanel } from "./CoachPanel";
import { DraftHelperPanel } from "./DraftHelperPanel";
import { useChampionNames } from "../hooks/useChampionNames";
import { championIconUrl, formatDuration } from "../lib/ddragon";
import { Swords, Clock, Shield, Ban } from "lucide-react";

interface Props {
  data: LiveGameData;
  myPuuid?: string;
}

export function LiveGameView({ data, myPuuid }: Props) {
  const champNames = useChampionNames();

  const phaseLabel =
    data.phase === "champ_select" ? "Выбор чемпионов" : "В игре";

  const timerPhaseLabel: Record<string, string> = {
    BAN_PICK: "Бан / Пик",
    PLANNING: "Планирование",
    FINALIZATION: "Финализация",
  };

  const timerText = data.timer
    ? `${timerPhaseLabel[data.timer.phase] ?? data.timer.phase} — ${Math.ceil(data.timer.timeLeftMs / 1000)}с`
    : null;

  const gameTimeText =
    data.gameTime != null && data.gameTime > 0
      ? formatDuration(data.gameTime)
      : null;

  const myBans = data.bans.filter((b) => b.teamId === 100);
  const enemyBans = data.bans.filter((b) => b.teamId === 200);

  return (
    <div className="flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-loss/20 text-loss text-sm font-semibold">
            <div className="w-2 h-2 rounded-full bg-loss animate-pulse" />
            LIVE
          </div>
          <h2 className="text-lg font-bold text-text-primary flex items-center gap-2">
            <Swords size={20} className="text-accent" />
            {phaseLabel}
          </h2>
        </div>
        <div className="flex items-center gap-3 text-text-muted text-sm">
          {timerText && (
            <span className="flex items-center gap-1.5">
              <Clock size={14} />
              {timerText}
            </span>
          )}
          {gameTimeText && (
            <span className="flex items-center gap-1.5">
              <Clock size={14} />
              {gameTimeText}
            </span>
          )}
        </div>
      </div>

      {/* Bans */}
      {(myBans.length > 0 || enemyBans.length > 0) && (
        <div className="flex items-center justify-between gap-4">
          <BanRow bans={myBans} champNames={champNames} label="Синие баны" />
          <Ban size={16} className="text-text-muted shrink-0" />
          <BanRow bans={enemyBans} champNames={champNames} label="Красные баны" reversed />
        </div>
      )}

      {/* Teams */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <TeamPanel
          title="Синяя команда"
          players={data.myTeam}
          champNames={champNames}
          myPuuid={myPuuid}
          teamColor="text-blue-400"
          borderColor="border-blue-500/30"
          phase={data.phase}
        />
        <TeamPanel
          title="Красная команда"
          players={data.enemyTeam}
          champNames={champNames}
          myPuuid={myPuuid}
          teamColor="text-red-400"
          borderColor="border-red-500/30"
          phase={data.phase}
        />
      </div>

      {/* Draft Helper (only during champ select) */}
      {data.phase === "champ_select" && <DraftHelperPanel />}

      {/* AI Coach */}
      <CoachPanel />
    </div>
  );
}

const ROLE_ORDER: Record<string, number> = {
  top: 0,
  jungle: 1,
  middle: 2,
  bottom: 3,
  utility: 4,
};

function sortByRole(players: LiveGameData["myTeam"]): LiveGameData["myTeam"] {
  return [...players].sort((a, b) => {
    const aOrder = ROLE_ORDER[a.assignedPosition?.toLowerCase() ?? ""] ?? 9;
    const bOrder = ROLE_ORDER[b.assignedPosition?.toLowerCase() ?? ""] ?? 9;
    return aOrder - bOrder;
  });
}

function TeamPanel({
  title,
  players,
  champNames,
  myPuuid,
  teamColor,
  borderColor,
  phase,
}: {
  title: string;
  players: LiveGameData["myTeam"];
  champNames: Record<number, string>;
  myPuuid?: string;
  teamColor: string;
  borderColor: string;
  phase: string;
}) {
  const sorted = phase === "in_game" ? sortByRole(players) : players;

  return (
    <div className={`rounded-xl border ${borderColor} bg-bg-secondary/50 p-4`}>
      <h3 className={`text-sm font-bold ${teamColor} mb-3 flex items-center gap-2`}>
        <Shield size={14} />
        {title}
      </h3>
      <div className="flex flex-col gap-2">
        {sorted.map((p, i) => (
          <LivePlayerRow
            key={p.puuid ?? `slot-${i}`}
            player={p}
            champNames={champNames}
            isMe={!!myPuuid && p.puuid === myPuuid}
          />
        ))}
        {sorted.length === 0 && (
          <p className="text-text-muted text-sm py-4 text-center">
            Ожидание игроков...
          </p>
        )}
      </div>
    </div>
  );
}

function BanRow({
  bans,
  champNames,
  label,
  reversed,
}: {
  bans: LiveGameData["bans"];
  champNames: Record<number, string>;
  label: string;
  reversed?: boolean;
}) {
  return (
    <div className={`flex items-center gap-1.5 ${reversed ? "flex-row-reverse" : ""}`}>
      {bans.map((b, i) => {
        const name = champNames[b.championId];
        return name ? (
          <div key={i} className="relative">
            <img
              src={championIconUrl(name)}
              alt={name}
              className="w-7 h-7 rounded grayscale opacity-50"
            />
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="w-full h-0.5 bg-loss rotate-45 rounded" />
            </div>
          </div>
        ) : (
          <div key={i} className="w-7 h-7 rounded bg-bg-primary" />
        );
      })}
      {bans.length === 0 && (
        <span className="text-xs text-text-muted">{label}</span>
      )}
    </div>
  );
}
