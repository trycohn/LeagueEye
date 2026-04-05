-- Initial schema for LeagueEye server

CREATE TABLE IF NOT EXISTS accounts (
    puuid           TEXT PRIMARY KEY,
    game_name       TEXT NOT NULL,
    tag_line        TEXT NOT NULL,
    profile_icon_id BIGINT,
    summoner_level  BIGINT,
    last_seen       BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS matches (
    match_id        TEXT NOT NULL,
    puuid           TEXT NOT NULL,
    champion_id     BIGINT,
    champion_name   TEXT,
    win             BOOLEAN,
    kills           INT,
    deaths          INT,
    assists         INT,
    cs              INT,
    gold            INT,
    damage          BIGINT,
    vision_score    INT,
    position        TEXT,
    game_duration   BIGINT,
    game_creation   BIGINT,
    queue_id        INT,
    items           INT[] DEFAULT '{}',
    summoner_spells INT[] DEFAULT '{}',
    lp_delta        INT,
    PRIMARY KEY (match_id, puuid)
);

CREATE INDEX IF NOT EXISTS idx_matches_puuid ON matches(puuid);

CREATE TABLE IF NOT EXISTS rank_snapshots (
    id          BIGSERIAL PRIMARY KEY,
    puuid       TEXT NOT NULL,
    queue_type  TEXT NOT NULL,
    tier        TEXT,
    rank        TEXT,
    lp          INT,
    wins        INT,
    losses      INT,
    recorded_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS match_participants (
    match_id         TEXT NOT NULL,
    puuid            TEXT NOT NULL,
    riot_id_name     TEXT,
    riot_id_tagline  TEXT,
    champion_id      BIGINT,
    champion_name    TEXT,
    champ_level      INT,
    team_id          INT,
    win              BOOLEAN,
    kills            INT,
    deaths           INT,
    assists          INT,
    cs               INT,
    gold             INT,
    damage           BIGINT,
    damage_taken     BIGINT,
    vision_score     INT,
    wards_placed     INT,
    wards_killed     INT,
    position         TEXT,
    items            INT[] DEFAULT '{}',
    summoner_spells  INT[] DEFAULT '{}',
    double_kills     INT DEFAULT 0,
    triple_kills     INT DEFAULT 0,
    quadra_kills     INT DEFAULT 0,
    penta_kills      INT DEFAULT 0,
    PRIMARY KEY (match_id, puuid)
);

CREATE INDEX IF NOT EXISTS idx_match_parts_match ON match_participants(match_id);

CREATE TABLE IF NOT EXISTS champion_mastery (
    puuid           TEXT NOT NULL,
    champion_id     BIGINT NOT NULL,
    champion_level  INT,
    champion_points BIGINT,
    updated_at      BIGINT NOT NULL,
    PRIMARY KEY (puuid, champion_id)
);
