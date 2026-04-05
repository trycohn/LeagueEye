use rusqlite::{Connection, Result as SqlResult, params};
use std::path::PathBuf;

use crate::models::{MatchDetail, MatchParticipantDetail, MatchSummary, PlayerProfile, RankInfo};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(db_path: PathBuf) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        let needs_recreate: bool = self.conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='matches'")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, String>(0)))
            .map(|sql| !sql.contains("PRIMARY KEY (match_id, puuid)"))
            .unwrap_or(false);
        if needs_recreate {
            let _ = self.conn.execute_batch("DROP TABLE IF EXISTS matches;");
        }

        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS accounts (
                puuid           TEXT PRIMARY KEY,
                game_name       TEXT NOT NULL,
                tag_line        TEXT NOT NULL,
                profile_icon_id INTEGER,
                summoner_level  INTEGER,
                last_seen       INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS matches (
                match_id        TEXT NOT NULL,
                puuid           TEXT NOT NULL,
                champion_id     INTEGER,
                champion_name   TEXT,
                win             INTEGER,
                kills           INTEGER,
                deaths          INTEGER,
                assists         INTEGER,
                cs              INTEGER,
                gold            INTEGER,
                damage          INTEGER,
                vision_score    INTEGER,
                position        TEXT,
                game_duration   INTEGER,
                game_creation   INTEGER,
                queue_id        INTEGER,
                items           TEXT,
                summoner_spells TEXT,
                lp_delta        INTEGER,
                PRIMARY KEY (match_id, puuid)
            );

            CREATE INDEX IF NOT EXISTS idx_matches_puuid ON matches(puuid);

            CREATE TABLE IF NOT EXISTS rank_snapshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                puuid       TEXT NOT NULL,
                queue_type  TEXT NOT NULL,
                tier        TEXT,
                rank        TEXT,
                lp          INTEGER,
                wins        INTEGER,
                losses      INTEGER,
                recorded_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS match_participants (
                match_id         TEXT NOT NULL,
                puuid            TEXT NOT NULL,
                riot_id_name     TEXT,
                riot_id_tagline  TEXT,
                champion_id      INTEGER,
                champion_name    TEXT,
                champ_level      INTEGER,
                team_id          INTEGER,
                win              INTEGER,
                kills            INTEGER,
                deaths           INTEGER,
                assists          INTEGER,
                cs               INTEGER,
                gold             INTEGER,
                damage           INTEGER,
                damage_taken     INTEGER,
                vision_score     INTEGER,
                wards_placed     INTEGER,
                wards_killed     INTEGER,
                position         TEXT,
                items            TEXT,
                summoner_spells  TEXT,
                double_kills     INTEGER,
                triple_kills     INTEGER,
                quadra_kills     INTEGER,
                penta_kills      INTEGER,
                PRIMARY KEY (match_id, puuid)
            );

            CREATE INDEX IF NOT EXISTS idx_match_parts_match ON match_participants(match_id);

            CREATE TABLE IF NOT EXISTS champion_mastery (
                puuid           TEXT NOT NULL,
                champion_id     INTEGER NOT NULL,
                champion_level  INTEGER,
                champion_points INTEGER,
                updated_at      INTEGER NOT NULL,
                PRIMARY KEY (puuid, champion_id)
            );
        ")?;

        // Миграция: добавить lp_delta если колонки нет
        let has_lp_delta: bool = self.conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='matches'")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, String>(0)))
            .map(|sql| sql.contains("lp_delta"))
            .unwrap_or(true);
        if !has_lp_delta {
            let _ = self.conn.execute_batch("ALTER TABLE matches ADD COLUMN lp_delta INTEGER;");
        }

        Ok(())
    }

    // --- Accounts ---

    pub fn save_account(&self, profile: &PlayerProfile) -> SqlResult<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR REPLACE INTO accounts
             (puuid, game_name, tag_line, profile_icon_id, summoner_level, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                profile.puuid,
                profile.game_name,
                profile.tag_line,
                profile.profile_icon_id,
                profile.summoner_level,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn get_last_account(&self) -> SqlResult<Option<StoredAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT puuid, game_name, tag_line, profile_icon_id, summoner_level
             FROM accounts ORDER BY last_seen DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(StoredAccount {
                puuid: row.get(0)?,
                game_name: row.get(1)?,
                tag_line: row.get(2)?,
                profile_icon_id: row.get(3)?,
                summoner_level: row.get(4)?,
            })
        })?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    // --- Matches ---

    pub fn get_cached_match_ids(&self, puuid: &str) -> SqlResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT match_id FROM matches WHERE puuid = ?1 ORDER BY game_creation DESC",
        )?;
        let ids = stmt
            .query_map(params![puuid], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    pub fn count_cached_matches(&self, puuid: &str) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM matches WHERE puuid = ?1",
            params![puuid],
            |row| row.get(0),
        )
    }

    pub fn get_cached_matches_paged(&self, puuid: &str, offset: usize, limit: usize) -> SqlResult<Vec<MatchSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT match_id, champion_id, champion_name, win, kills, deaths, assists,
                    cs, gold, damage, vision_score, position,
                    game_duration, game_creation, queue_id, items, summoner_spells, lp_delta
             FROM matches WHERE puuid = ?1
             ORDER BY game_creation DESC LIMIT ?2 OFFSET ?3",
        )?;
        let matches = stmt
            .query_map(params![puuid, limit as i64, offset as i64], |row| {
                Ok(MatchSummary {
                    match_id: row.get(0)?,
                    champion_id: row.get(1)?,
                    champion_name: row.get(2)?,
                    win: row.get::<_, i32>(3)? != 0,
                    kills: row.get(4)?,
                    deaths: row.get(5)?,
                    assists: row.get(6)?,
                    cs: row.get(7)?,
                    gold: row.get(8)?,
                    damage: row.get(9)?,
                    vision_score: row.get(10)?,
                    position: row.get(11)?,
                    game_duration: row.get(12)?,
                    game_creation: row.get(13)?,
                    queue_id: row.get(14)?,
                    items: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                    summoner_spells: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
                    lp_delta: row.get(17)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(matches)
    }

    pub fn get_cached_matches(&self, puuid: &str, limit: usize) -> SqlResult<Vec<MatchSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT match_id, champion_id, champion_name, win, kills, deaths, assists,
                    cs, gold, damage, vision_score, position,
                    game_duration, game_creation, queue_id, items, summoner_spells, lp_delta
             FROM matches WHERE puuid = ?1
             ORDER BY game_creation DESC LIMIT ?2",
        )?;
        let matches = stmt
            .query_map(params![puuid, limit as i64], |row| {
                Ok(MatchSummary {
                    match_id: row.get(0)?,
                    champion_id: row.get(1)?,
                    champion_name: row.get(2)?,
                    win: row.get::<_, i32>(3)? != 0,
                    kills: row.get(4)?,
                    deaths: row.get(5)?,
                    assists: row.get(6)?,
                    cs: row.get(7)?,
                    gold: row.get(8)?,
                    damage: row.get(9)?,
                    vision_score: row.get(10)?,
                    position: row.get(11)?,
                    game_duration: row.get(12)?,
                    game_creation: row.get(13)?,
                    queue_id: row.get(14)?,
                    items: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                    summoner_spells: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
                    lp_delta: row.get(17)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(matches)
    }

    pub fn save_matches(&self, puuid: &str, matches: &[MatchSummary]) -> SqlResult<()> {
        for m in matches {
            self.conn.execute(
                "INSERT OR IGNORE INTO matches
                 (match_id, puuid, champion_id, champion_name, win, kills, deaths, assists,
                  cs, gold, damage, vision_score, position,
                  game_duration, game_creation, queue_id, items, summoner_spells, lp_delta)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
                params![
                    m.match_id,
                    puuid,
                    m.champion_id,
                    m.champion_name,
                    m.win as i32,
                    m.kills,
                    m.deaths,
                    m.assists,
                    m.cs,
                    m.gold,
                    m.damage,
                    m.vision_score,
                    m.position,
                    m.game_duration,
                    m.game_creation,
                    m.queue_id,
                    serde_json::to_string(&m.items).unwrap_or_default(),
                    serde_json::to_string(&m.summoner_spells).unwrap_or_default(),
                    m.lp_delta,
                ],
            )?;
        }
        Ok(())
    }

    // --- Match participants (full game data) ---

    pub fn save_match_participants(&self, match_id: &str, game_duration: i64, participants: &[MatchParticipantDetail]) -> SqlResult<()> {
        let _ = game_duration;
        for p in participants {
            self.conn.execute(
                "INSERT OR IGNORE INTO match_participants
                 (match_id, puuid, riot_id_name, riot_id_tagline, champion_id, champion_name,
                  champ_level, team_id, win, kills, deaths, assists, cs, gold, damage,
                  damage_taken, vision_score, wards_placed, wards_killed, position,
                  items, summoner_spells, double_kills, triple_kills, quadra_kills, penta_kills)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
                params![
                    match_id,
                    p.puuid,
                    p.riot_id_name,
                    p.riot_id_tagline,
                    p.champion_id,
                    p.champion_name,
                    p.champ_level,
                    p.team_id,
                    p.win as i32,
                    p.kills,
                    p.deaths,
                    p.assists,
                    p.cs,
                    p.gold,
                    p.damage,
                    p.damage_taken,
                    p.vision_score,
                    p.wards_placed,
                    p.wards_killed,
                    p.position,
                    serde_json::to_string(&p.items).unwrap_or_default(),
                    serde_json::to_string(&p.summoner_spells).unwrap_or_default(),
                    p.double_kills,
                    p.triple_kills,
                    p.quadra_kills,
                    p.penta_kills,
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_match_detail(&self, match_id: &str) -> SqlResult<Option<MatchDetail>> {
        let meta = self.conn.prepare(
            "SELECT game_duration, game_creation, queue_id FROM matches WHERE match_id = ?1 LIMIT 1"
        )?.query_row(params![match_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i32>(2)?))
        });

        let (game_duration, game_creation, queue_id) = match meta {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let mut stmt = self.conn.prepare(
            "SELECT puuid, riot_id_name, riot_id_tagline, champion_id, champion_name,
                    champ_level, team_id, win, kills, deaths, assists, cs, gold, damage,
                    damage_taken, vision_score, wards_placed, wards_killed, position,
                    items, summoner_spells, double_kills, triple_kills, quadra_kills, penta_kills
             FROM match_participants WHERE match_id = ?1
             ORDER BY team_id ASC"
        )?;

        let participants: Vec<MatchParticipantDetail> = stmt
            .query_map(params![match_id], |row| {
                Ok(MatchParticipantDetail {
                    puuid: row.get(0)?,
                    riot_id_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    riot_id_tagline: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    champion_id: row.get(3)?,
                    champion_name: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    champ_level: row.get::<_, Option<i32>>(5)?.unwrap_or(1),
                    team_id: row.get::<_, Option<i32>>(6)?.unwrap_or(100),
                    win: row.get::<_, i32>(7).unwrap_or(0) != 0,
                    kills: row.get(8)?,
                    deaths: row.get(9)?,
                    assists: row.get(10)?,
                    cs: row.get(11)?,
                    gold: row.get(12)?,
                    damage: row.get(13)?,
                    damage_taken: row.get::<_, Option<i64>>(14)?.unwrap_or(0),
                    vision_score: row.get::<_, Option<i32>>(15)?.unwrap_or(0),
                    wards_placed: row.get::<_, Option<i32>>(16)?.unwrap_or(0),
                    wards_killed: row.get::<_, Option<i32>>(17)?.unwrap_or(0),
                    position: row.get::<_, Option<String>>(18)?.unwrap_or_default(),
                    items: serde_json::from_str(&row.get::<_, String>(19).unwrap_or_default()).unwrap_or_default(),
                    summoner_spells: serde_json::from_str(&row.get::<_, String>(20).unwrap_or_default()).unwrap_or_default(),
                    double_kills: row.get::<_, Option<i32>>(21)?.unwrap_or(0),
                    triple_kills: row.get::<_, Option<i32>>(22)?.unwrap_or(0),
                    quadra_kills: row.get::<_, Option<i32>>(23)?.unwrap_or(0),
                    penta_kills: row.get::<_, Option<i32>>(24)?.unwrap_or(0),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        if participants.is_empty() {
            return Ok(None);
        }

        Ok(Some(MatchDetail {
            match_id: match_id.to_string(),
            game_duration,
            game_creation,
            queue_id,
            participants,
        }))
    }

    // --- Rank snapshots ---

    pub fn get_latest_ranks(&self, puuid: &str) -> SqlResult<Vec<RankInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT queue_type, tier, rank, lp, wins, losses
             FROM rank_snapshots
             WHERE puuid = ?1 AND id IN (
                 SELECT MAX(id) FROM rank_snapshots WHERE puuid = ?1 GROUP BY queue_type
             )"
        )?;
        let rows = stmt.query_map(params![puuid], |row| {
            let wins: i32 = row.get(4)?;
            let losses: i32 = row.get(5)?;
            let total = wins + losses;
            Ok(RankInfo {
                queue_type: row.get(0)?,
                tier: row.get(1)?,
                rank: row.get(2)?,
                lp: row.get(3)?,
                wins,
                losses,
                winrate: if total > 0 { ((wins as f64 / total as f64) * 100.0).round() } else { 0.0 },
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    /// Возвращает LP из ближайшего rank_snapshot для RANKED_SOLO_5x5.
    /// `before=true` — ищет последний снэпшот ДО timestamp,
    /// `before=false` — первый снэпшот ПОСЛЕ timestamp.
    pub fn get_lp_at(&self, puuid: &str, timestamp_ms: i64, before: bool) -> Option<i32> {
        let (sql, param) = if before {
            (
                "SELECT lp FROM rank_snapshots
                 WHERE puuid = ?1 AND queue_type = 'RANKED_SOLO_5x5' AND recorded_at <= ?2
                 ORDER BY recorded_at DESC LIMIT 1",
                timestamp_ms,
            )
        } else {
            (
                "SELECT lp FROM rank_snapshots
                 WHERE puuid = ?1 AND queue_type = 'RANKED_SOLO_5x5' AND recorded_at >= ?2
                 ORDER BY recorded_at ASC LIMIT 1",
                timestamp_ms,
            )
        };
        self.conn
            .prepare(sql)
            .ok()?
            .query_row(params![puuid, param], |row| row.get(0))
            .ok()
    }

    pub fn save_rank_snapshot(&self, puuid: &str, ranks: &[RankInfo]) -> SqlResult<()> {
        let now = now_ms();
        for r in ranks {
            self.conn.execute(
                "INSERT INTO rank_snapshots
                 (puuid, queue_type, tier, rank, lp, wins, losses, recorded_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![puuid, r.queue_type, r.tier, r.rank, r.lp, r.wins, r.losses, now],
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: i64,
    pub summoner_level: i64,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
