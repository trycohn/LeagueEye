ALTER TABLE match_reviews
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'ready';

ALTER TABLE match_reviews
    ADD COLUMN IF NOT EXISTS error_text TEXT;

ALTER TABLE match_reviews
    ADD COLUMN IF NOT EXISTS updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT);

UPDATE match_reviews
SET status = 'ready',
    error_text = NULL,
    updated_at = COALESCE(updated_at, created_at)
WHERE status IS DISTINCT FROM 'ready'
   OR updated_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_match_reviews_status_updated_at
    ON match_reviews(status, updated_at);
