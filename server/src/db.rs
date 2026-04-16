use sqlx::PgPool;
use leagueeye_shared::models::*;

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // --- Accounts ---

    pub async fn save_account(&self, profile: &PlayerProfile) -> Result<(), sqlx::Error> {
        let now = now_ms();
        sqlx::query(
            "INSERT INTO accounts (puuid, game_name, tag_line, profile_icon_id, summoner_level, last_seen)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (puuid) DO UPDATE SET
                game_name = EXCLUDED.game_name,
                tag_line = EXCLUDED.tag_line,
                profile_icon_id = EXCLUDED.profile_icon_id,
                summoner_level = EXCLUDED.summoner_level,
                last_seen = EXCLUDED.last_seen"
        )
        .bind(&profile.puuid)
        .bind(&profile.game_name)
        .bind(&profile.tag_line)
        .bind(profile.profile_icon_id)
        .bind(profile.summoner_level)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_last_account(&self) -> Result<Option<StoredAccount>, sqlx::Error> {
        let row = sqlx::query_as::<_, StoredAccountRow>(
            "SELECT puuid, game_name, tag_line, profile_icon_id, summoner_level
             FROM accounts ORDER BY last_seen DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.into()))
    }

    // --- Matches ---

    pub async fn get_cached_match_ids(&self, puuid: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT match_id FROM matches WHERE puuid = $1 ORDER BY game_creation DESC"
        )
        .bind(puuid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn count_cached_matches(&self, puuid: &str) -> Result<i64, sqlx::Error> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM matches WHERE puuid = $1"
        )
        .bind(puuid)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn get_cached_matches_paged(&self, puuid: &str, offset: i64, limit: i64) -> Result<Vec<MatchSummary>, sqlx::Error> {
        let rows: Vec<MatchSummaryRow> = sqlx::query_as(
            "SELECT match_id, champion_id, champion_name, win, kills, deaths, assists,
                    cs, gold, damage, vision_score, position,
                    game_duration, game_creation, queue_id, items, summoner_spells, lp_delta
             FROM matches WHERE puuid = $1
             ORDER BY game_creation DESC LIMIT $2 OFFSET $3"
        )
        .bind(puuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn save_matches(&self, puuid: &str, matches: &[MatchSummary]) -> Result<(), sqlx::Error> {
        for m in matches {
            sqlx::query(
                "INSERT INTO matches
                 (match_id, puuid, champion_id, champion_name, win, kills, deaths, assists,
                  cs, gold, damage, vision_score, position,
                  game_duration, game_creation, queue_id, items, summoner_spells, lp_delta)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
                 ON CONFLICT (match_id, puuid) DO UPDATE SET
                    lp_delta = COALESCE(matches.lp_delta, EXCLUDED.lp_delta)"
            )
            .bind(&m.match_id)
            .bind(puuid)
            .bind(m.champion_id)
            .bind(&m.champion_name)
            .bind(m.win)
            .bind(m.kills)
            .bind(m.deaths)
            .bind(m.assists)
            .bind(m.cs)
            .bind(m.gold)
            .bind(m.damage)
            .bind(m.vision_score)
            .bind(&m.position)
            .bind(m.game_duration)
            .bind(m.game_creation)
            .bind(m.queue_id)
            .bind(&m.items)
            .bind(&m.summoner_spells)
            .bind(m.lp_delta)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // --- Match participants ---

    pub async fn save_match_participants(&self, match_id: &str, participants: &[MatchParticipantDetail]) -> Result<(), sqlx::Error> {
        for p in participants {
            sqlx::query(
                "INSERT INTO match_participants
                 (match_id, puuid, riot_id_name, riot_id_tagline, champion_id, champion_name,
                  champ_level, team_id, win, kills, deaths, assists, cs, gold, damage,
                  damage_taken, vision_score, wards_placed, wards_killed, position,
                  items, summoner_spells, double_kills, triple_kills, quadra_kills, penta_kills)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26)
                 ON CONFLICT (match_id, puuid) DO NOTHING"
            )
            .bind(match_id)
            .bind(&p.puuid)
            .bind(&p.riot_id_name)
            .bind(&p.riot_id_tagline)
            .bind(p.champion_id)
            .bind(&p.champion_name)
            .bind(p.champ_level)
            .bind(p.team_id)
            .bind(p.win)
            .bind(p.kills)
            .bind(p.deaths)
            .bind(p.assists)
            .bind(p.cs)
            .bind(p.gold)
            .bind(p.damage)
            .bind(p.damage_taken)
            .bind(p.vision_score)
            .bind(p.wards_placed)
            .bind(p.wards_killed)
            .bind(&p.position)
            .bind(&p.items)
            .bind(&p.summoner_spells)
            .bind(p.double_kills)
            .bind(p.triple_kills)
            .bind(p.quadra_kills)
            .bind(p.penta_kills)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_match_detail(&self, match_id: &str) -> Result<Option<MatchDetail>, sqlx::Error> {
        let meta: Option<(i64, i64, i32)> = sqlx::query_as(
            "SELECT game_duration, game_creation, queue_id FROM matches WHERE match_id = $1 LIMIT 1"
        )
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;

        let (game_duration, game_creation, queue_id) = match meta {
            Some(m) => m,
            None => return Ok(None),
        };

        let rows: Vec<MatchParticipantRow> = sqlx::query_as(
            "SELECT puuid, riot_id_name, riot_id_tagline, champion_id, champion_name,
                    champ_level, team_id, win, kills, deaths, assists, cs, gold, damage,
                    damage_taken, vision_score, wards_placed, wards_killed, position,
                    items, summoner_spells, double_kills, triple_kills, quadra_kills, penta_kills
             FROM match_participants WHERE match_id = $1
             ORDER BY team_id ASC"
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let participants: Vec<MatchParticipantDetail> = rows.into_iter().map(|r| r.into()).collect();

        Ok(Some(MatchDetail {
            match_id: match_id.to_string(),
            game_duration,
            game_creation,
            queue_id,
            participants,
        }))
    }

    // --- Rank snapshots ---

    pub async fn get_latest_ranks(&self, puuid: &str) -> Result<Vec<RankInfo>, sqlx::Error> {
        let rows: Vec<RankRow> = sqlx::query_as(
            "SELECT queue_type, tier, rank, lp, wins, losses
             FROM rank_snapshots
             WHERE puuid = $1 AND id IN (
                 SELECT MAX(id) FROM rank_snapshots WHERE puuid = $1 GROUP BY queue_type
             )"
        )
        .bind(puuid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_lp_at(&self, puuid: &str, timestamp_ms: i64, before: bool) -> Result<Option<i32>, sqlx::Error> {
        let row: Option<(i32,)> = if before {
            sqlx::query_as(
                "SELECT lp FROM rank_snapshots
                 WHERE puuid = $1 AND queue_type = 'RANKED_SOLO_5x5' AND recorded_at <= $2
                 ORDER BY recorded_at DESC LIMIT 1"
            )
            .bind(puuid)
            .bind(timestamp_ms)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT lp FROM rank_snapshots
                 WHERE puuid = $1 AND queue_type = 'RANKED_SOLO_5x5' AND recorded_at >= $2
                 ORDER BY recorded_at ASC LIMIT 1"
            )
            .bind(puuid)
            .bind(timestamp_ms)
            .fetch_optional(&self.pool)
            .await?
        };
        Ok(row.map(|r| r.0))
    }

    pub async fn save_rank_snapshot(&self, puuid: &str, ranks: &[RankInfo]) -> Result<(), sqlx::Error> {
        let now = now_ms();
        for r in ranks {
            sqlx::query(
                "INSERT INTO rank_snapshots
                 (puuid, queue_type, tier, rank, lp, wins, losses, recorded_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"
            )
            .bind(puuid)
            .bind(&r.queue_type)
            .bind(&r.tier)
            .bind(&r.rank)
            .bind(r.lp)
            .bind(r.wins)
            .bind(r.losses)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
    /// Compute and backfill lp_delta for ranked solo matches that have NULL lp_delta.
    /// Uses rank_snapshots recorded before game start and after game end.
    pub async fn backfill_lp_deltas(&self, puuid: &str) -> Result<(), sqlx::Error> {
        // queue_id 420 = Ranked Solo/Duo
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT match_id, game_creation, game_duration
             FROM matches
             WHERE puuid = $1 AND queue_id = 420 AND lp_delta IS NULL
             ORDER BY game_creation DESC"
        )
        .bind(puuid)
        .fetch_all(&self.pool)
        .await?;

        for (match_id, game_creation, game_duration) in rows {
            let game_end_ms = game_creation + game_duration * 1000;
            let before = self.get_lp_at(puuid, game_creation, true).await?;
            let after = self.get_lp_at(puuid, game_end_ms, false).await?;

            if let (Some(lp_before), Some(lp_after)) = (before, after) {
                let delta = lp_after - lp_before;
                sqlx::query(
                    "UPDATE matches SET lp_delta = $1 WHERE match_id = $2 AND puuid = $3"
                )
                .bind(delta)
                .bind(&match_id)
                .bind(puuid)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    // --- Matchups ---

    pub async fn get_matchups(&self, puuid: &str) -> Result<Vec<MatchupStat>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                enemy.champion_id   AS enemy_champion_id,
                enemy.champion_name  AS enemy_champion_name,
                me.position          AS position,
                COUNT(*)::INT        AS games,
                SUM(CASE WHEN me.win THEN 1 ELSE 0 END)::INT AS wins,
                (SUM(CASE WHEN me.win THEN 1 ELSE 0 END)::FLOAT / COUNT(*) * 100) AS winrate,
                (SUM(me.kills)::FLOAT   / COUNT(*)) AS avg_kills,
                (SUM(me.deaths)::FLOAT  / COUNT(*)) AS avg_deaths,
                (SUM(me.assists)::FLOAT / COUNT(*)) AS avg_assists
            FROM matches me
            JOIN match_participants enemy
              ON enemy.match_id = me.match_id
             AND enemy.position = me.position
             AND enemy.team_id <> (
                 SELECT mp2.team_id FROM match_participants mp2
                 WHERE mp2.match_id = me.match_id AND mp2.puuid = me.puuid
                 LIMIT 1
             )
            WHERE me.puuid = $1
              AND me.position IS NOT NULL
              AND me.position <> ''
              AND me.position <> 'UNKNOWN'
              AND enemy.champion_name IS NOT NULL
            GROUP BY enemy.champion_id, enemy.champion_name, me.position
            HAVING COUNT(*) >= 2
            ORDER BY games DESC, winrate DESC
            "#
        )
        .bind(puuid)
        .fetch_all(&self.pool)
        .await?;

        use sqlx::Row;
        let stats = rows.into_iter().map(|r| {
            MatchupStat {
                enemy_champion_id: r.try_get::<i64, _>("enemy_champion_id").unwrap_or(0),
                enemy_champion_name: r.try_get::<Option<String>, _>("enemy_champion_name").unwrap_or_default().unwrap_or_default(),
                position: r.try_get::<Option<String>, _>("position").unwrap_or_default().unwrap_or_default(),
                games: r.try_get::<i32, _>("games").unwrap_or(0),
                wins: r.try_get::<i32, _>("wins").unwrap_or(0),
                winrate: {
                    let w: f64 = r.try_get("winrate").unwrap_or(0.0);
                    (w * 10.0).round() / 10.0
                },
                avg_kills: {
                    let v: f64 = r.try_get("avg_kills").unwrap_or(0.0);
                    (v * 10.0).round() / 10.0
                },
                avg_deaths: {
                    let v: f64 = r.try_get("avg_deaths").unwrap_or(0.0);
                    (v * 10.0).round() / 10.0
                },
                avg_assists: {
                    let v: f64 = r.try_get("avg_assists").unwrap_or(0.0);
                    (v * 10.0).round() / 10.0
                },
            }
        }).collect();

        Ok(stats)
    }

    // --- Frequent Teammates ---

    pub async fn get_frequent_teammates(&self, puuid: &str) -> Result<Vec<FrequentTeammate>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                mp.puuid,
                mp.riot_id_name,
                mp.riot_id_tagline,
                COUNT(*)::INT AS games_together,
                SUM(CASE WHEN mp.win THEN 1 ELSE 0 END)::INT AS wins_together
            FROM matches me
            JOIN match_participants mp
              ON mp.match_id = me.match_id
             AND mp.team_id = (
                 SELECT mp2.team_id FROM match_participants mp2
                 WHERE mp2.match_id = me.match_id AND mp2.puuid = me.puuid
                 LIMIT 1
             )
             AND mp.puuid <> me.puuid
            WHERE me.puuid = $1
            GROUP BY mp.puuid, mp.riot_id_name, mp.riot_id_tagline
            HAVING COUNT(*) >= 3
            ORDER BY games_together DESC
            LIMIT 10
            "#
        )
        .bind(puuid)
        .fetch_all(&self.pool)
        .await?;

        use sqlx::Row;
        let teammates = rows.into_iter().map(|r| {
            let games: i32 = r.try_get("games_together").unwrap_or(0);
            let wins: i32 = r.try_get("wins_together").unwrap_or(0);
            let winrate = if games > 0 {
                ((wins as f64 / games as f64) * 1000.0).round() / 10.0
            } else {
                0.0
            };
            FrequentTeammate {
                puuid: r.try_get::<String, _>("puuid").unwrap_or_default(),
                game_name: r.try_get::<Option<String>, _>("riot_id_name").unwrap_or_default().unwrap_or_default(),
                tag_line: r.try_get::<Option<String>, _>("riot_id_tagline").unwrap_or_default().unwrap_or_default(),
                games_together: games,
                wins_together: wins,
                winrate,
            }
        }).collect();

        Ok(teammates)
    }

    // --- Match Reviews ---

    pub async fn get_review(&self, match_id: &str, puuid: &str) -> Result<Option<PostGameReview>, sqlx::Error> {
        let row: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT match_id, puuid, review_text, created_at FROM match_reviews WHERE match_id = $1 AND puuid = $2"
        )
        .bind(match_id)
        .bind(puuid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(match_id, puuid, review_text, created_at)| PostGameReview {
            match_id,
            puuid,
            review_text,
            created_at,
        }))
    }

    pub async fn save_review(&self, match_id: &str, puuid: &str, text: &str) -> Result<(), sqlx::Error> {
        let now = now_ms() / 1000; // seconds
        sqlx::query(
            "INSERT INTO match_reviews (match_id, puuid, review_text, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (match_id, puuid) DO UPDATE SET review_text = EXCLUDED.review_text, created_at = EXCLUDED.created_at"
        )
        .bind(match_id)
        .bind(puuid)
        .bind(text)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_review(&self, match_id: &str, puuid: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM match_reviews WHERE match_id = $1 AND puuid = $2")
            .bind(match_id)
            .bind(puuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Match Timelines ---

    pub async fn get_timeline(&self, match_id: &str) -> Result<Option<serde_json::Value>, sqlx::Error> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT timeline_json FROM match_timelines WHERE match_id = $1"
        )
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn save_timeline(&self, match_id: &str, timeline_json: &serde_json::Value) -> Result<(), sqlx::Error> {
        let now = now_ms() / 1000;
        sqlx::query(
            "INSERT INTO match_timelines (match_id, timeline_json, created_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (match_id) DO NOTHING"
        )
        .bind(match_id)
        .bind(timeline_json)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Champion Stats for a specific puuid (for review context) ---

    pub async fn get_champion_stats_for_player(&self, puuid: &str) -> Result<Vec<ChampionStat>, sqlx::Error> {
        let matches = self.get_cached_matches_paged(puuid, 0, 500).await?;
        Ok(build_champion_stats(&matches))
    }

    pub async fn get_global_dashboard_data(&self) -> Result<GlobalDashboardData, sqlx::Error> {
        // 1. Global Stats
        let total_players: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&self.pool)
            .await?;

        let analyzed_matches: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT match_id)::BIGINT FROM matches")
            .fetch_one(&self.pool)
            .await?;

        let hours_played: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(game_duration)::BIGINT / 3600, 0) FROM matches")
            .fetch_one(&self.pool)
            .await?;

        let pentakills: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(penta_kills)::BIGINT, 0) FROM match_participants")
            .fetch_one(&self.pool)
            .await?;

        let stats = GlobalStats {
            total_players,
            analyzed_matches,
            hours_played,
            pentakills,
        };

        // 2. Best Players by Role (Top, Jungle, Mid, ADC, Support)
        let roles = vec![
            ("Top", "TOP"),
            ("Jungle", "JUNGLE"),
            ("Mid", "MIDDLE"),
            ("ADC", "BOTTOM"),
            ("Support", "UTILITY"),
        ];

        let mut best_by_role = Vec::new();
        for (display_role, db_role) in roles {
            let row = sqlx::query(
                r#"
                SELECT 
                    riot_id_name as player,
                    riot_id_tagline as tag,
                    champion_name as champ,
                    COUNT(*) as games_played,
                    (SUM(CASE WHEN win = true THEN 1 ELSE 0 END)::FLOAT / COUNT(*) * 100) as winrate,
                    ((SUM(kills) + SUM(assists))::FLOAT / GREATEST(SUM(deaths), 1)) as kda
                FROM match_participants
                WHERE position = $1 AND riot_id_name IS NOT NULL
                GROUP BY puuid, riot_id_name, riot_id_tagline, champion_name
                HAVING COUNT(*) >= 3
                ORDER BY winrate DESC, kda DESC
                LIMIT 1
                "#
            )
            .bind(db_role)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(r) = row {
                use sqlx::Row;
                let player: Option<String> = r.try_get("player").unwrap_or_default();
                let tag: Option<String> = r.try_get("tag").unwrap_or_default();
                let champ: Option<String> = r.try_get("champ").unwrap_or_default();
                let winrate: Option<f64> = r.try_get("winrate").unwrap_or_default();
                let kda: Option<f64> = r.try_get("kda").unwrap_or_default();

                best_by_role.push(BestPlayerRole {
                    role: display_role.to_string(),
                    player: player.unwrap_or_default(),
                    tag: tag.unwrap_or_default(),
                    champ: champ.unwrap_or_default(),
                    winrate: format!("{:.1}%", winrate.unwrap_or(0.0)),
                    kda: format!("{:.1}", kda.unwrap_or(0.0)),
                });
            }
        }

        // 3. Top Winrate Champions
        let top_winrates_rows = sqlx::query(
            r#"
            SELECT 
                champion_name as champ,
                COUNT(*) as games,
                (SUM(CASE WHEN win = true THEN 1 ELSE 0 END)::FLOAT / COUNT(*) * 100) as winrate
            FROM match_participants
            WHERE champion_name IS NOT NULL
            GROUP BY champion_id, champion_name
            HAVING COUNT(*) >= 5
            ORDER BY winrate DESC
            LIMIT 5
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let top_winrates = top_winrates_rows.into_iter().map(|r| {
            use sqlx::Row;
            let champ: Option<String> = r.try_get("champ").unwrap_or_default();
            let winrate: Option<f64> = r.try_get("winrate").unwrap_or_default();
            let games: Option<i64> = r.try_get("games").unwrap_or_default();

            TopWinrateChampion {
                champ: champ.unwrap_or_default(),
                winrate: format!("{:.1}%", winrate.unwrap_or(0.0)),
                games: games.unwrap_or(0),
            }
        }).collect();

        Ok(GlobalDashboardData {
            stats,
            best_by_role,
            top_winrates,
        })
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// --- sqlx row types ---

#[derive(sqlx::FromRow)]
struct StoredAccountRow {
    puuid: String,
    game_name: String,
    tag_line: String,
    profile_icon_id: Option<i64>,
    summoner_level: Option<i64>,
}

impl From<StoredAccountRow> for StoredAccount {
    fn from(r: StoredAccountRow) -> Self {
        StoredAccount {
            puuid: r.puuid,
            game_name: r.game_name,
            tag_line: r.tag_line,
            profile_icon_id: r.profile_icon_id.unwrap_or(0),
            summoner_level: r.summoner_level.unwrap_or(0),
        }
    }
}

#[derive(sqlx::FromRow)]
struct MatchSummaryRow {
    match_id: String,
    champion_id: Option<i64>,
    champion_name: Option<String>,
    win: Option<bool>,
    kills: Option<i32>,
    deaths: Option<i32>,
    assists: Option<i32>,
    cs: Option<i32>,
    gold: Option<i32>,
    damage: Option<i64>,
    vision_score: Option<i32>,
    position: Option<String>,
    game_duration: Option<i64>,
    game_creation: Option<i64>,
    queue_id: Option<i32>,
    items: Option<Vec<i32>>,
    summoner_spells: Option<Vec<i32>>,
    lp_delta: Option<i32>,
}

impl From<MatchSummaryRow> for MatchSummary {
    fn from(r: MatchSummaryRow) -> Self {
        MatchSummary {
            match_id: r.match_id,
            champion_id: r.champion_id.unwrap_or(0),
            champion_name: r.champion_name.unwrap_or_default(),
            win: r.win.unwrap_or(false),
            kills: r.kills.unwrap_or(0),
            deaths: r.deaths.unwrap_or(0),
            assists: r.assists.unwrap_or(0),
            cs: r.cs.unwrap_or(0),
            gold: r.gold.unwrap_or(0),
            damage: r.damage.unwrap_or(0),
            vision_score: r.vision_score.unwrap_or(0),
            position: r.position.unwrap_or_default(),
            game_duration: r.game_duration.unwrap_or(0),
            game_creation: r.game_creation.unwrap_or(0),
            queue_id: r.queue_id.unwrap_or(0),
            items: r.items.unwrap_or_default(),
            summoner_spells: r.summoner_spells.unwrap_or_default(),
            lp_delta: r.lp_delta,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MatchParticipantRow {
    puuid: Option<String>,
    riot_id_name: Option<String>,
    riot_id_tagline: Option<String>,
    champion_id: Option<i64>,
    champion_name: Option<String>,
    champ_level: Option<i32>,
    team_id: Option<i32>,
    win: Option<bool>,
    kills: Option<i32>,
    deaths: Option<i32>,
    assists: Option<i32>,
    cs: Option<i32>,
    gold: Option<i32>,
    damage: Option<i64>,
    damage_taken: Option<i64>,
    vision_score: Option<i32>,
    wards_placed: Option<i32>,
    wards_killed: Option<i32>,
    position: Option<String>,
    items: Option<Vec<i32>>,
    summoner_spells: Option<Vec<i32>>,
    double_kills: Option<i32>,
    triple_kills: Option<i32>,
    quadra_kills: Option<i32>,
    penta_kills: Option<i32>,
}

impl From<MatchParticipantRow> for MatchParticipantDetail {
    fn from(r: MatchParticipantRow) -> Self {
        MatchParticipantDetail {
            puuid: r.puuid.unwrap_or_default(),
            riot_id_name: r.riot_id_name.unwrap_or_default(),
            riot_id_tagline: r.riot_id_tagline.unwrap_or_default(),
            champion_id: r.champion_id.unwrap_or(0),
            champion_name: r.champion_name.unwrap_or_default(),
            champ_level: r.champ_level.unwrap_or(1),
            team_id: r.team_id.unwrap_or(100),
            win: r.win.unwrap_or(false),
            kills: r.kills.unwrap_or(0),
            deaths: r.deaths.unwrap_or(0),
            assists: r.assists.unwrap_or(0),
            cs: r.cs.unwrap_or(0),
            gold: r.gold.unwrap_or(0),
            damage: r.damage.unwrap_or(0),
            damage_taken: r.damage_taken.unwrap_or(0),
            vision_score: r.vision_score.unwrap_or(0),
            wards_placed: r.wards_placed.unwrap_or(0),
            wards_killed: r.wards_killed.unwrap_or(0),
            position: r.position.unwrap_or_default(),
            items: r.items.unwrap_or_default(),
            summoner_spells: r.summoner_spells.unwrap_or_default(),
            double_kills: r.double_kills.unwrap_or(0),
            triple_kills: r.triple_kills.unwrap_or(0),
            quadra_kills: r.quadra_kills.unwrap_or(0),
            penta_kills: r.penta_kills.unwrap_or(0),
        }
    }
}

#[derive(sqlx::FromRow)]
struct RankRow {
    queue_type: String,
    tier: Option<String>,
    rank: Option<String>,
    lp: Option<i32>,
    wins: Option<i32>,
    losses: Option<i32>,
}

impl From<RankRow> for RankInfo {
    fn from(r: RankRow) -> Self {
        let wins = r.wins.unwrap_or(0);
        let losses = r.losses.unwrap_or(0);
        let total = wins + losses;
        RankInfo {
            queue_type: r.queue_type,
            tier: r.tier.unwrap_or_default(),
            rank: r.rank.unwrap_or_default(),
            lp: r.lp.unwrap_or(0),
            wins,
            losses,
            winrate: if total > 0 { ((wins as f64 / total as f64) * 100.0).round() } else { 0.0 },
        }
    }
}
