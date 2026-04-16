CREATE TABLE IF NOT EXISTS match_reviews (
    match_id TEXT NOT NULL,
    puuid TEXT NOT NULL,
    review_text TEXT NOT NULL,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    PRIMARY KEY (match_id, puuid)
);

CREATE TABLE IF NOT EXISTS match_timelines (
    match_id TEXT PRIMARY KEY,
    timeline_json JSONB NOT NULL,
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
);
