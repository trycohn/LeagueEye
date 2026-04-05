import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, Loader2 } from "lucide-react";
import type { MatchSummary, MatchDetail } from "../lib/types";
import {
  championIconUrl,
  itemIconUrl,
  positionIconUrl,
  formatDuration,
  timeAgo,
} from "../lib/ddragon";
import { MatchDetailView } from "./MatchDetailView";

const POSITION_LABEL: Record<string, string> = {
  TOP: "Top",
  JUNGLE: "Jungle",
  MIDDLE: "Mid",
  BOTTOM: "ADC",
  UTILITY: "Support",
};

interface Props {
  match: MatchSummary;
  onPlayerClick: (gameName: string, tagLine: string) => void;
}

export function MatchCard({ match: m, onPlayerClick }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [detail, setDetail] = useState<MatchDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);

  const kda =
    m.deaths === 0
      ? "Perfect"
      : ((m.kills + m.assists) / m.deaths).toFixed(1);

  const csPerMin =
    m.gameDuration > 0
      ? ((m.cs / m.gameDuration) * 60).toFixed(1)
      : "0";

  async function toggleExpand() {
    if (expanded) {
      setExpanded(false);
      return;
    }
    if (detail) {
      setExpanded(true);
      return;
    }
    setLoadingDetail(true);
    try {
      const d = await invoke<MatchDetail>("get_match_detail", { matchId: m.matchId });
      setDetail(d);
      setExpanded(true);
    } catch (e) {
      console.error("get_match_detail error:", e);
    } finally {
      setLoadingDetail(false);
    }
  }

  return (
    <div className="rounded-lg border transition-colors overflow-hidden"
      style={{
        backgroundColor: m.win ? "rgba(34,197,94,0.05)" : "rgba(239,68,68,0.05)",
        borderColor: m.win ? "rgba(34,197,94,0.2)" : "rgba(239,68,68,0.2)",
      }}
    >
      <div
        className={`flex items-center gap-3 p-3 cursor-pointer transition-colors ${
          m.win ? "hover:bg-win/10" : "hover:bg-loss/10"
        }`}
        onClick={toggleExpand}
      >
        <div
          className={`w-1 self-stretch rounded-full ${
            m.win ? "bg-win" : "bg-loss"
          }`}
        />

        <img
          src={championIconUrl(m.championName)}
          alt={m.championName}
          className="w-10 h-10 rounded-lg"
        />

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-sm text-text-primary">
              {m.championName}
            </span>
            {m.position && POSITION_LABEL[m.position.toUpperCase()] && (
              <img
                src={positionIconUrl(m.position.toLowerCase())}
                alt={POSITION_LABEL[m.position.toUpperCase()]}
                title={POSITION_LABEL[m.position.toUpperCase()]}
                className="w-4 h-4 opacity-70 brightness-200"
              />
            )}
          </div>
          <div className="flex items-center gap-1.5 text-xs text-text-secondary mt-0.5">
            <span className={m.win ? "text-win font-medium" : "text-loss font-medium"}>
              {m.win ? "Победа" : "Поражение"}
            </span>
            <span className="text-text-muted">·</span>
            <span>{formatDuration(m.gameDuration)}</span>
            <span className="text-text-muted">·</span>
            <span>{timeAgo(m.gameCreation)}</span>
          </div>
        </div>

        <div className="text-center min-w-[80px]">
          <div className="text-sm font-bold text-text-primary">
            {m.kills}/{m.deaths}/{m.assists}
          </div>
          <div className="text-xs text-text-muted">
            KDA{" "}
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

        <div className="text-center min-w-[60px]">
          <div className="text-sm text-text-primary">{m.cs}</div>
          <div className="text-[10px] text-text-muted">{csPerMin} cs/m</div>
        </div>

        <div className="text-center min-w-[45px]">
          {m.lpDelta != null ? (
            <span
              className={`text-xs font-bold ${
                m.lpDelta > 0 ? "text-win" : m.lpDelta < 0 ? "text-loss" : "text-text-muted"
              }`}
            >
              {m.lpDelta > 0 ? `+${m.lpDelta}` : m.lpDelta} LP
            </span>
          ) : (
            <span className="text-xs text-text-muted">?LP</span>
          )}
        </div>

        <div className="hidden sm:flex gap-0.5">
          {m.items.map((itemId, i) =>
            itemId > 0 ? (
              <img
                key={i}
                src={itemIconUrl(itemId)}
                alt=""
                className="w-6 h-6 rounded"
              />
            ) : (
              <div
                key={i}
                className="w-6 h-6 rounded bg-bg-secondary border border-border"
              />
            )
          )}
        </div>

        <div className="flex items-center ml-1">
          {loadingDetail ? (
            <Loader2 size={16} className="animate-spin text-text-muted" />
          ) : (
            <ChevronDown
              size={16}
              className={`text-text-muted transition-transform duration-200 ${
                expanded ? "rotate-180" : ""
              }`}
            />
          )}
        </div>
      </div>

      {expanded && detail && (
        <div className="border-t border-border/50">
          <MatchDetailView detail={detail} onPlayerClick={onPlayerClick} />
        </div>
      )}
    </div>
  );
}
