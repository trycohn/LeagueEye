export interface PlayerProfile {
  puuid: string;
  gameName: string;
  tagLine: string;
  summonerLevel: number;
  profileIconId: number;
  ranked: RankInfo[];
}

export interface RankInfo {
  queueType: string;
  tier: string;
  rank: string;
  lp: number;
  wins: number;
  losses: number;
  winrate: number;
}

export interface MasteryInfo {
  championId: number;
  championLevel: number;
  championPoints: number;
}

export interface MatchSummary {
  matchId: string;
  championName: string;
  championId: number;
  win: boolean;
  kills: number;
  deaths: number;
  assists: number;
  cs: number;
  gold: number;
  damage: number;
  visionScore: number;
  position: string;
  gameDuration: number;
  gameCreation: number;
  queueId: number;
  items: number[];
  summonerSpells: number[];
  lpDelta: number | null;
}

export interface DetectedAccount {
  puuid: string;
  gameName: string;
  tagLine: string;
  profileIconId: number;
  summonerLevel: number;
  ranked: RankInfo[];
}

export interface StoredAccount {
  puuid: string;
  gameName: string;
  tagLine: string;
  profileIconId: number;
  summonerLevel: number;
}

// --- Live Game ---

export interface LiveGameData {
  phase: "champ_select" | "in_game" | "none";
  queueId: number | null;
  myTeam: LivePlayer[];
  enemyTeam: LivePlayer[];
  bans: LiveBan[];
  gameTime: number | null;
  timer: LiveTimer | null;
}

export interface LivePlayer {
  puuid: string | null;
  gameName: string | null;
  tagLine: string | null;
  championId: number;
  assignedPosition: string | null;
  spell1Id: number;
  spell2Id: number;
  teamId: number;
  rank: RankInfo | null;
  isPicking: boolean;
}

export interface LiveBan {
  championId: number;
  teamId: number;
}

export interface LiveTimer {
  phase: string;
  timeLeftMs: number;
}

export interface MatchParticipantDetail {
  puuid: string;
  riotIdName: string;
  riotIdTagline: string;
  championId: number;
  championName: string;
  champLevel: number;
  teamId: number;
  win: boolean;
  kills: number;
  deaths: number;
  assists: number;
  cs: number;
  gold: number;
  damage: number;
  damageTaken: number;
  visionScore: number;
  wardsPlaced: number;
  wardsKilled: number;
  position: string;
  items: number[];
  summonerSpells: number[];
  doubleKills: number;
  tripleKills: number;
  quadraKills: number;
  pentaKills: number;
}

export interface MatchDetail {
  matchId: string;
  gameDuration: number;
  gameCreation: number;
  queueId: number;
  participants: MatchParticipantDetail[];
}

export interface MatchesAndStats {
  matches: MatchSummary[];
  championStats: ChampionStat[];
  totalCached: number;
  totalWins: number;
  totalLosses: number;
}

export interface ChampionStat {
  championId: number;
  championName: string;
  games: number;
  wins: number;
  winrate: number;
  avgKills: number;
  avgDeaths: number;
  avgAssists: number;
  avgCs: number;
  position: string;
}

// --- Global Dashboard ---

export interface GlobalStats {
  totalPlayers: number;
  analyzedMatches: number;
  hoursPlayed: number;
  pentakills: number;
}

export interface BestPlayerRole {
  role: string;
  player: string;
  tag: string;
  champ: string;
  winrate: string;
  kda: string;
}

export interface TopWinrateChampion {
  champ: string;
  winrate: string;
  games: number;
}

export interface GlobalDashboardData {
  stats: GlobalStats;
  bestByRole: BestPlayerRole[];
  topWinrates: TopWinrateChampion[];
}

// --- AI Coach ---

export interface CoachStreamPayload {
  kind: "start" | "delta" | "end" | "error" | "draft-start" | "draft-delta" | "draft-end" | "draft-error";
  text: string | null;
}

export interface CoachMessage {
  text: string;
  timestamp: number;
}

// --- Draft Helper ---

export interface ChampionPoolEntry {
  championName: string;
  games: number;
  winrate: number;
}

// --- Gold Comparison ---

export interface GoldComparisonData {
  lanes: LaneGoldComparison[];
  gameTime: number | null;
}

export interface LaneGoldComparison {
  role: string;
  allyChampionName: string;
  allyGold: number;
  enemyChampionName: string;
  enemyGold: number;
  goldDiff: number;
}

// --- Matchup Stats ---

export interface MatchupStat {
  enemyChampionId: number;
  enemyChampionName: string;
  position: string;
  games: number;
  wins: number;
  winrate: number;
  avgKills: number;
  avgDeaths: number;
  avgAssists: number;
}

// --- Updates ---

export interface UpdateInfo {
  version: string;
  body: string | null;
  date: string | null;
}
