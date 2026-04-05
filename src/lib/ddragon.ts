const DDRAGON_VERSION = "16.7.1";
const DDRAGON_BASE = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}`;

export function championIconUrl(championName: string): string {
  return `${DDRAGON_BASE}/img/champion/${championName}.png`;
}

export function profileIconUrl(iconId: number): string {
  return `${DDRAGON_BASE}/img/profileicon/${iconId}.png`;
}

export function itemIconUrl(itemId: number): string {
  if (itemId === 0) return "";
  return `${DDRAGON_BASE}/img/item/${itemId}.png`;
}

export function summonerSpellIconUrl(spellId: number): string {
  const spellMap: Record<number, string> = {
    1: "SummonerBoost",
    3: "SummonerExhaust",
    4: "SummonerFlash",
    6: "SummonerHaste",
    7: "SummonerHeal",
    11: "SummonerSmite",
    12: "SummonerTeleport",
    13: "SummonerMana",
    14: "SummonerDot",
    21: "SummonerBarrier",
    32: "SummonerSnowball",
  };
  const name = spellMap[spellId] || "SummonerFlash";
  return `${DDRAGON_BASE}/img/spell/${name}.png`;
}

const TIER_ORDER: Record<string, number> = {
  IRON: 0,
  BRONZE: 1,
  SILVER: 2,
  GOLD: 3,
  PLATINUM: 4,
  EMERALD: 5,
  DIAMOND: 6,
  MASTER: 7,
  GRANDMASTER: 8,
  CHALLENGER: 9,
};

export function tierColor(tier: string): string {
  const colors: Record<string, string> = {
    IRON: "#6b7280",
    BRONZE: "#b45309",
    SILVER: "#9ca3af",
    GOLD: "#eab308",
    PLATINUM: "#06b6d4",
    EMERALD: "#10b981",
    DIAMOND: "#818cf8",
    MASTER: "#a855f7",
    GRANDMASTER: "#ef4444",
    CHALLENGER: "#f59e0b",
  };
  return colors[tier.toUpperCase()] || "#94a3b8";
}

export function tierDisplayName(tier: string): string {
  const names: Record<string, string> = {
    IRON: "Iron",
    BRONZE: "Bronze",
    SILVER: "Silver",
    GOLD: "Gold",
    PLATINUM: "Platinum",
    EMERALD: "Emerald",
    DIAMOND: "Diamond",
    MASTER: "Master",
    GRANDMASTER: "Grandmaster",
    CHALLENGER: "Challenger",
  };
  return names[tier.toUpperCase()] || tier;
}

export function queueName(queueType: string): string {
  const names: Record<string, string> = {
    RANKED_SOLO_5x5: "Solo/Duo",
    RANKED_FLEX_SR: "Flex 5v5",
    RANKED_TFT: "TFT",
  };
  return names[queueType] || queueType;
}

export function positionName(position: string): string {
  const names: Record<string, string> = {
    TOP: "Top",
    JUNGLE: "Jungle",
    MIDDLE: "Mid",
    BOTTOM: "ADC",
    UTILITY: "Support",
  };
  return names[position] || position;
}

const CDRAGON_BASE = "https://raw.communitydragon.org/latest/plugins";

export function positionIconUrl(position: string): string {
  const key = position.toLowerCase();
  const map: Record<string, string> = {
    top: "position-top",
    jungle: "position-jungle",
    middle: "position-middle",
    bottom: "position-bottom",
    utility: "position-utility",
  };
  const file = map[key] || "position-middle";
  return `${CDRAGON_BASE}/rcp-fe-lol-champ-select/global/default/svg/${file}.svg`;
}

export function rankEmblemUrl(tier: string): string {
  const t = tier.toLowerCase();
  return `${CDRAGON_BASE}/rcp-fe-lol-static-assets/global/default/images/ranked-emblem/emblem-${t}.png`;
}

export function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function timeAgo(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}д назад`;
  if (hours > 0) return `${hours}ч назад`;
  if (minutes > 0) return `${minutes}м назад`;
  return "только что";
}

export function formatPoints(points: number): string {
  if (points >= 1_000_000) return `${(points / 1_000_000).toFixed(1)}M`;
  if (points >= 1_000) return `${(points / 1_000).toFixed(1)}K`;
  return points.toString();
}

export { TIER_ORDER };
