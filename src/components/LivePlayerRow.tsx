import type { LivePlayer } from "../lib/types";
import { championIconUrl, tierColor, tierDisplayName, positionIconUrl, rankEmblemUrl } from "../lib/ddragon";

interface Props {
  player: LivePlayer;
  champNames: Record<number, string>;
  isMe?: boolean;
}

const POSITION_LABEL: Record<string, string> = {
  top: "Top",
  jungle: "Jungle",
  middle: "Mid",
  bottom: "ADC",
  utility: "Support",
};

export function LivePlayerRow({ player, champNames, isMe }: Props) {
  const champName = champNames[player.championId];
  const hasChamp = player.championId > 0 && champName;

  const pos = player.assignedPosition?.toLowerCase() ?? "";

  const rank = player.rank;
  const displayName = player.gameName
    ? `${player.gameName}#${player.tagLine ?? ""}`
    : null;

  return (
    <div
      className={`flex items-center gap-3 px-3 py-2 rounded-lg transition-all ${
        player.isPicking
          ? "ring-2 ring-accent animate-pulse bg-accent/10"
          : "bg-bg-secondary"
      } ${isMe ? "border border-accent/50" : "border border-transparent"}`}
    >
      {/* Champion icon */}
      <div className="relative shrink-0">
        {hasChamp ? (
          <img
            src={championIconUrl(champName)}
            alt={champName}
            className="w-10 h-10 rounded-lg"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = "none";
            }}
          />
        ) : (
          <div className="w-10 h-10 rounded-lg bg-bg-primary flex items-center justify-center text-text-muted text-lg">
            ?
          </div>
        )}
        {player.isPicking && (
          <div className="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-accent animate-ping" />
        )}
      </div>

      {/* Position */}
      <div className="w-8 h-8 shrink-0 flex items-center justify-center" title={POSITION_LABEL[pos] ?? ""}>
        {pos && POSITION_LABEL[pos] ? (
          <img
            src={positionIconUrl(pos)}
            alt={POSITION_LABEL[pos]}
            className="w-5 h-5 opacity-70 brightness-200"
          />
        ) : (
          <span className="text-xs text-text-muted">—</span>
        )}
      </div>

      {/* Name + champion */}
      <div className="flex-1 min-w-0">
        {displayName ? (
          <p className={`text-sm font-medium truncate ${isMe ? "text-accent" : "text-text-primary"}`}>
            {displayName}
          </p>
        ) : (
          <p className="text-sm text-text-muted italic truncate">Неизвестный</p>
        )}
        {hasChamp && (
          <p className="text-xs text-text-muted truncate">{champName}</p>
        )}
      </div>

      {/* Rank */}
      <div className="shrink-0 flex items-center gap-2">
        {rank ? (
          <>
            <img
              src={rankEmblemUrl(rank.tier)}
              alt={rank.tier}
              className="w-7 h-7"
              onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
            />
            <div className="text-right">
              <p className="text-sm font-semibold" style={{ color: tierColor(rank.tier) }}>
                {tierDisplayName(rank.tier)} {rank.rank}
              </p>
              <p className="text-xs text-text-muted">
                {rank.lp} LP ·{" "}
                <span className={rank.winrate >= 50 ? "text-win" : "text-loss"}>
                  {rank.winrate}%
                </span>
              </p>
            </div>
          </>
        ) : (
          <p className="text-xs text-text-muted">Unranked</p>
        )}
      </div>
    </div>
  );
}
