-- Week 3: initial schema for crypton-identity

create extension if not exists "pgcrypto";

-- Users table: one row per registered human
create table if not exists users (
    id          uuid        primary key default gen_random_uuid(),
    username    text        not null unique,
    created_at  timestamptz not null default now()
);

-- Credentials table: one row per passkey device
-- A user can have many credentials (multi-device)
create table if not exists credentials (
    id              uuid    primary key default gen_random_uuid(),
    user_id         uuid    not null references users(id) on delete cascade,

    credential_id   bytea   not null unique,   -- WebAuthn credential ID (raw bytes)
    public_key      bytea   not null,           -- COSE-encoded public key
    sign_count      bigint  not null default 0, -- replay protection counter

    nickname        text,                       -- e.g. "MacBook Touch ID"
    status          text    not null default 'active', -- active | revoked
    created_at      timestamptz not null default now(),
    last_used_at    timestamptz
);

create index if not exists idx_credentials_user_id on credentials(user_id);
create index if not exists idx_credentials_status  on credentials(status);
