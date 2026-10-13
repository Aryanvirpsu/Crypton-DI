-- P2 policy additions
-- 1. Allow 'pending_approval' credential status (new_device policy)
ALTER TABLE credentials DROP CONSTRAINT credentials_status_check;
ALTER TABLE credentials ADD CONSTRAINT credentials_status_check
    CHECK (status IN ('active', 'lost', 'revoked', 'pending_approval'));

-- 2. Index to make pending-device admin query fast
CREATE INDEX IF NOT EXISTS idx_credentials_pending
    ON credentials(status) WHERE status = 'pending_approval';
