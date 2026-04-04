-- Recovery requests table: tracks self-service recovery workflows.
-- method: trusted_device (approved by another passkey) | admin (approved by admin action)
-- status: pending -> approved -> completed | rejected

CREATE TABLE IF NOT EXISTS recovery_requests (
    id                       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                  UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status                   TEXT        NOT NULL DEFAULT 'pending',
    method                   TEXT        NOT NULL DEFAULT 'trusted_device',
    approved_by_credential_id UUID,      -- credential that approved; no FK so history survives revocation
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at               TIMESTAMPTZ NOT NULL,

    CONSTRAINT recovery_status_check CHECK (status   IN ('pending', 'approved', 'completed', 'rejected')),
    CONSTRAINT recovery_method_check CHECK (method   IN ('trusted_device', 'admin'))
);

CREATE INDEX IF NOT EXISTS idx_recovery_user_status ON recovery_requests(user_id, status);
