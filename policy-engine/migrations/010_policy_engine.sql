-- Expand role CHECK constraint to include security_analyst and viewer
ALTER TABLE users DROP CONSTRAINT users_role_check;
ALTER TABLE users ADD CONSTRAINT users_role_check
    CHECK (role IN ('user', 'admin', 'super_admin', 'security_analyst', 'viewer'));

-- Runtime-configurable policy table
CREATE TABLE IF NOT EXISTS policies (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    key          TEXT        NOT NULL UNIQUE,
    display_name TEXT        NOT NULL,
    description  TEXT        NOT NULL DEFAULT '',
    enabled      BOOLEAN     NOT NULL DEFAULT false,
    config       JSONB       NOT NULL DEFAULT '{}',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_policies_key ON policies(key);

-- Seed the 8 known policies (all disabled by default — operators opt in)
INSERT INTO policies (key, display_name, description, config) VALUES
    ('geo_block',       'Geographic Block',       'Block logins from high-risk countries',           '{"blocked_countries":["CN","RU","KP","IR"]}'),
    ('stepup_risk',     'Step-Up on High Risk',   'Step-up auth when risk score exceeds threshold',  '{"threshold":70}'),
    ('tor_block',       'TOR/VPN Block',           'Block logins via TOR exit nodes and known VPNs', '{}'),
    ('geo_velocity',    'Geo Velocity Check',      'Deny impossible travel within time window',       '{"window_hours":4}'),
    ('device_trust',    'Device Trust Timeout',    'Require re-verification after device inactivity', '{"inactivity_days":30}'),
    ('failed_attempts', 'Failed Attempt Lock',     'Lock account after repeated failed logins',       '{"max_failures":5,"window_minutes":15}'),
    ('off_hours',       'Off-Hours Login Notify',  'Alert on logins outside business hours',          '{"start_hour":8,"end_hour":18,"timezone":"UTC"}'),
    ('new_device',      'New Device Approval',     'Require admin approval for new device enrollment','{}')
ON CONFLICT (key) DO NOTHING;

-- Policy evaluation audit trail
CREATE TABLE IF NOT EXISTS policy_evaluations (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        REFERENCES users(id) ON DELETE SET NULL,
    policy_key TEXT        NOT NULL,
    decision   TEXT        NOT NULL,   -- 'allow' | 'deny' | 'stepup'
    reason     TEXT,
    context    JSONB       NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_policy_eval_user    ON policy_evaluations(user_id);
CREATE INDEX IF NOT EXISTS idx_policy_eval_created ON policy_evaluations(created_at DESC);

-- Failed login tracking for the failed_attempts evaluator
CREATE TABLE IF NOT EXISTS login_attempts (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID        REFERENCES users(id) ON DELETE CASCADE,
    username     TEXT        NOT NULL,
    ip           TEXT,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    success      BOOLEAN     NOT NULL DEFAULT false
);

CREATE INDEX IF NOT EXISTS idx_login_attempts_user_time
    ON login_attempts(user_id, attempted_at DESC);
CREATE INDEX IF NOT EXISTS idx_login_attempts_username_time
    ON login_attempts(username, attempted_at DESC);

-- Auto-update policies.updated_at on every PATCH
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER policies_updated_at
    BEFORE UPDATE ON policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
