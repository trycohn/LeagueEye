use rusqlite::{Connection, Result as SqlResult, params};
use std::path::PathBuf;

use crate::models::{PlayerProfile, StoredAccount};

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
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS accounts (
                puuid           TEXT PRIMARY KEY,
                game_name       TEXT NOT NULL,
                tag_line        TEXT NOT NULL,
                profile_icon_id INTEGER,
                summoner_level  INTEGER,
                last_seen       INTEGER NOT NULL
            );
        ")?;
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
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
