-- Server-side authorization state for admin boundaries and recovery enrollment.

ALTER TABLE users
ADD COLUMN IF NOT EXISTS role text NOT NULL DEFAULT 'user';

ALTER TABLE users
ADD CONSTRAINT users_role_check
CHECK (role IN ('user', 'admin', 'super_admin'));

ALTER TABLE recovery_requests
ADD COLUMN IF NOT EXISTS public_token_hash text;
