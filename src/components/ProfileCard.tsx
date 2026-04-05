import type { PlayerProfile } from "../lib/types";
import { profileIconUrl } from "../lib/ddragon";
import { RankBadge } from "./RankBadge";

interface Props {
  profile: PlayerProfile;
}

export function ProfileCard({ profile }: Props) {
  const soloRank = profile.ranked.find(
    (r) => r.queueType === "RANKED_SOLO_5x5"
  );
  const flexRank = profile.ranked.find(
    (r) => r.queueType === "RANKED_FLEX_SR"
  );

  return (
    <div className="p-5 rounded-xl bg-bg-card border border-border">
      <div className="flex items-center gap-4 mb-4">
        <div className="relative shrink-0">
          <img
            src={profileIconUrl(profile.profileIconId)}
            alt="icon"
            className="w-20 h-20 rounded-xl border-2 border-border"
          />
          <span className="absolute -bottom-1.5 left-1/2 -translate-x-1/2 bg-bg-secondary text-xs px-2 py-0.5 rounded-full border border-border text-text-secondary font-medium">
            {profile.summonerLevel}
          </span>
        </div>

        <div>
          <h2 className="text-xl font-bold text-text-primary">
            {profile.gameName}
            <span className="text-text-muted font-normal">
              #{profile.tagLine}
            </span>
          </h2>
        </div>
      </div>

      <div className="flex flex-col gap-2 items-start">
        {soloRank && <RankBadge rank={soloRank} />}
        {flexRank && <RankBadge rank={flexRank} />}
        {!soloRank && !flexRank && (
          <div className="text-text-muted text-sm py-2">Unranked</div>
        )}
      </div>
    </div>
  );
}
