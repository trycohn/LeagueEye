use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;

use crate::models::{FavoritePlayer, PlayerProfile, StoredAccount};

/// Minimal local SQLite — only caches the last account for instant startup.
/// All match data, ranks, and mastery are now on the server.
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
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS accounts (
                puuid           TEXT PRIMARY KEY,
                game_name       TEXT NOT NULL,
                tag_line        TEXT NOT NULL,
                profile_icon_id INTEGER,
                summoner_level  INTEGER,
                last_seen       INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS overlay_positions (
                overlay_id TEXT PRIMARY KEY,
                x          INTEGER NOT NULL,
                y          INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS favorites (
                puuid           TEXT PRIMARY KEY,
                game_name       TEXT NOT NULL,
                tag_line        TEXT NOT NULL,
                profile_icon_id INTEGER NOT NULL DEFAULT 0,
                added_at        INTEGER NOT NULL,
                source          TEXT NOT NULL DEFAULT 'manual'
            );
        ",
        )?;
        Ok(())
    }

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

    pub fn save_overlay_position(&self, overlay_id: &str, x: i32, y: i32) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO overlay_positions (overlay_id, x, y) VALUES (?1, ?2, ?3)",
            params![overlay_id, x, y],
        )?;
        Ok(())
    }

    pub fn get_overlay_position(&self, overlay_id: &str) -> SqlResult<Option<(i32, i32)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT x, y FROM overlay_positions WHERE overlay_id = ?1")?;
        let mut rows = stmt.query_map(params![overlay_id], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?))
        })?;
        Ok(rows.next().and_then(|r| r.ok()))
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

    // ─── Favorites ──────────────────────────────────────────────────────────

    pub fn get_favorites(&self) -> SqlResult<Vec<FavoritePlayer>> {
        let mut stmt = self.conn.prepare(
            "SELECT puuid, game_name, tag_line, profile_icon_id, added_at, source
             FROM favorites ORDER BY added_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FavoritePlayer {
                puuid: row.get(0)?,
                game_name: row.get(1)?,
                tag_line: row.get(2)?,
                profile_icon_id: row.get(3)?,
                added_at: row.get(4)?,
                source: row.get(5)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn add_favorite(
        &self,
        puuid: &str,
        game_name: &str,
        tag_line: &str,
        profile_icon_id: i64,
        source: &str,
    ) -> SqlResult<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT OR REPLACE INTO favorites
             (puuid, game_name, tag_line, profile_icon_id, added_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![puuid, game_name, tag_line, profile_icon_id, now, source],
        )?;
        Ok(())
    }

    pub fn remove_favorite(&self, puuid: &str) -> SqlResult<()> {
        self.conn
            .execute("DELETE FROM favorites WHERE puuid = ?1", params![puuid])?;
        Ok(())
    }

    pub fn is_favorite(&self, puuid: &str) -> SqlResult<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM favorites WHERE puuid = ?1",
            params![puuid],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
