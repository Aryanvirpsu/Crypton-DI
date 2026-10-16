-- Assessment inquiries (Agent Floor lead intake).
--
-- Design (see services/crypton-main/api/README.md):
--   * Everything lives in a private schema, `assessment`, which Supabase's public API
--     (PostgREST) does not expose. Nothing here is reachable through the anon key.
--   * RLS is enabled on every table. Only the operator role has policies. Anonymous and
--     authenticated users have no policy AND no grant, so each layer denies on its own.
--   * The web tier never touches the tables. It logs in as `assessment_writer`, whose ONLY
--     privilege is EXECUTE on assessment.submit_inquiry(): one atomic call that applies the
--     per-client throttle, the per-email throttle and idempotency under concurrency.
--   * Contact details and the system description are encrypted by the application
--     (AES-256-GCM) before they reach the database; the database holds ciphertext only.
--
-- Roles are created NOLOGIN. Set their passwords out of band, never in a migration:
--   alter role assessment_writer   login password '<generated>';
--   alter role assessment_operator login password '<generated>';

begin;

create schema if not exists assessment;

-- -- roles --------------------------------------------------------------------------------
do $$
begin
  if not exists (select from pg_roles where rolname = 'assessment_writer') then
    create role assessment_writer nologin noinherit;
  end if;
  if not exists (select from pg_roles where rolname = 'assessment_operator') then
    create role assessment_operator nologin noinherit;
  end if;
end
$$;

-- A runaway or malicious request must not hold the database: bound every session.
alter role assessment_writer set statement_timeout = '5s';
alter role assessment_writer set lock_timeout = '2s';
alter role assessment_writer set idle_in_transaction_session_timeout = '10s';
alter role assessment_writer connection limit 30;
alter role assessment_operator set statement_timeout = '30s';

-- -- schema access: nobody but the two roles above ----------------------------------------
revoke all on schema assessment from public;
do $$
declare r text;
begin
  foreach r in array array['anon', 'authenticated', 'service_role'] loop
    if exists (select from pg_roles where rolname = r) then
      execute format('revoke all on schema assessment from %I', r);
    end if;
  end loop;
end
$$;
grant usage on schema assessment to assessment_writer, assessment_operator;

-- -- inquiries ----------------------------------------------------------------------------
create table assessment.assessment_inquiries (
  id                  uuid primary key default gen_random_uuid(),

  -- Server-side idempotency. The key is the UUID inside the signed challenge token the
  -- server issued, so a client cannot choose it. One key, one row, however many retries.
  idempotency_key     uuid not null unique,
  request_fingerprint text not null check (request_fingerprint ~ '^[a-f0-9]{64}$'),

  status              text not null default 'new'
                        check (status in ('new', 'reviewing', 'contacted', 'qualified', 'closed', 'spam')),

  -- name, email, organization and the optional system description, as one AES-256-GCM
  -- envelope: 12-byte IV | 16-byte tag | ciphertext. The row's idempotency key is bound in
  -- as additional authenticated data, so a ciphertext cannot be moved to another row.
  payload             bytea not null check (octet_length(payload) between 29 and 16384),
  key_id              text not null check (key_id ~ '^[a-f0-9]{8}$'),

  -- Keyed hash of the lower-cased email: throttling and de-duplication without decrypting.
  email_hash          text not null check (email_hash ~ '^[a-f0-9]{64}$'),

  consent             boolean not null check (consent is true),
  privacy_version     text not null check (char_length(privacy_version) between 1 and 40),
  source              text not null check (source = 'agent-floor'),

  created_at          timestamptz not null default now(),
  updated_at          timestamptz not null default now()
);

create index assessment_inquiries_email_recent on assessment.assessment_inquiries (email_hash, created_at desc);
create index assessment_inquiries_created on assessment.assessment_inquiries (created_at);
create index assessment_inquiries_status on assessment.assessment_inquiries (status, created_at desc);

comment on table assessment.assessment_inquiries is
  'Agent Floor assessment requests. payload is application-encrypted; readable only by assessment_operator.';

create function assessment.touch_updated_at() returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.updated_at := now();
  return new;
end
$$;

create trigger assessment_inquiries_touch
  before update on assessment.assessment_inquiries
  for each row execute function assessment.touch_updated_at();

-- -- shared throttle counters -------------------------------------------------------------
-- Replaces the old process-local/SQLite counters. Shared by every concurrent instance.
create table assessment.rate_limits (
  bucket     text primary key,
  count      integer not null check (count > 0),
  expires_at timestamptz not null
);
create index rate_limits_expiry on assessment.rate_limits (expires_at);

-- -- row level security -------------------------------------------------------------------
alter table assessment.assessment_inquiries enable row level security;
alter table assessment.rate_limits enable row level security;

-- The operator may read inquiries and move their status along. No other role has a policy.
create policy operator_read on assessment.assessment_inquiries
  for select to assessment_operator using (true);
create policy operator_update on assessment.assessment_inquiries
  for update to assessment_operator using (true) with check (true);

-- -- table privileges: deny by default, then grant the minimum ---------------------------
revoke all on all tables in schema assessment from public;
do $$
declare r text;
begin
  foreach r in array array['anon', 'authenticated', 'service_role'] loop
    if exists (select from pg_roles where rolname = r) then
      execute format('revoke all on all tables in schema assessment from %I', r);
      execute format('revoke all on all functions in schema assessment from %I', r);
    end if;
  end loop;
end
$$;

grant select on assessment.assessment_inquiries to assessment_operator;
-- Column-level: the operator can change a lead's status and nothing else about it.
grant update (status) on assessment.assessment_inquiries to assessment_operator;

-- -- maintenance --------------------------------------------------------------------------
-- Retention (90 days) and expired throttle rows. Called opportunistically by submit_inquiry
-- and safe to schedule (pg_cron) as well. Bounded so a single call is always cheap.
create function assessment.prune_expired(p_retention interval default interval '90 days')
returns integer
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_deleted integer;
begin
  delete from assessment.rate_limits where expires_at < now();

  with doomed as (
    select id from assessment.assessment_inquiries
    where created_at < now() - p_retention
    order by created_at
    limit 500
  )
  delete from assessment.assessment_inquiries i using doomed d where i.id = d.id;
  get diagnostics v_deleted = row_count;
  return v_deleted;
end
$$;

revoke all on function assessment.prune_expired(interval) from public;

-- -- the one write path -------------------------------------------------------------------
-- One atomic call. Returns the HTTP status the API should answer with:
--   201 stored            200 identical retry (same receipt)    409 same key, different request
--   429 throttled (per client or per email)
-- Concurrent callers are safe: the per-email advisory lock serialises the email throttle,
-- and the unique idempotency key resolves any remaining race.
create function assessment.submit_inquiry(
  p_idempotency_key uuid,
  p_fingerprint     text,
  p_email_hash      text,
  p_ip_hash         text,
  p_key_id          text,
  p_payload         bytea,
  p_privacy_version text,
  p_source          text
) returns table (http_status integer, outcome text, inquiry_id uuid)
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_minute   bigint := floor(extract(epoch from now()) / 60);
  v_count    integer;
  v_existing record;
  v_id       uuid;
begin
  if p_ip_hash !~ '^[a-f0-9]{64}$' then
    raise exception 'invalid client hash' using errcode = '22023';
  end if;

  -- Housekeeping on a small fraction of calls, so no scheduler is required.
  if random() < 0.02 then
    perform assessment.prune_expired();
  end if;

  -- 1. Per-client throttle: 60 calls per minute, counted on every call.
  insert into assessment.rate_limits as r (bucket, count, expires_at)
  values ('ip:' || p_ip_hash || ':' || v_minute, 1, now() + interval '2 minutes')
  on conflict (bucket) do update set count = r.count + 1
  returning r.count into v_count;

  if v_count > 60 then
    return query select 429, 'rate-limit'::text, null::uuid;
    return;
  end if;

  -- 2. Serialise everything that shares an email address (throttle + insert).
  perform pg_advisory_xact_lock(hashtextextended(p_email_hash, 0));

  -- 3. Idempotent retry of a request we already stored.
  select i.id, i.request_fingerprint into v_existing
  from assessment.assessment_inquiries i
  where i.idempotency_key = p_idempotency_key;

  if found then
    if v_existing.request_fingerprint = p_fingerprint then
      return query select 200, 'received'::text, v_existing.id;
    else
      return query select 409, 'changed-retry'::text, null::uuid;
    end if;
    return;
  end if;

  -- 4. Per-email throttle: 3 accepted requests per hour.
  select count(*) into v_count
  from assessment.assessment_inquiries i
  where i.email_hash = p_email_hash and i.created_at > now() - interval '1 hour';

  if v_count >= 3 then
    return query select 429, 'rate-limit'::text, null::uuid;
    return;
  end if;

  -- 5. Store it. A unique violation means a concurrent identical request won the race.
  begin
    insert into assessment.assessment_inquiries
      (idempotency_key, request_fingerprint, payload, key_id, email_hash, consent, privacy_version, source)
    values
      (p_idempotency_key, p_fingerprint, p_payload, p_key_id, p_email_hash, true, p_privacy_version, p_source)
    returning id into v_id;
  exception when unique_violation then
    select i.id, i.request_fingerprint into v_existing
    from assessment.assessment_inquiries i
    where i.idempotency_key = p_idempotency_key;

    if found and v_existing.request_fingerprint = p_fingerprint then
      return query select 200, 'received'::text, v_existing.id;
    else
      return query select 409, 'changed-retry'::text, null::uuid;
    end if;
    return;
  end;

  return query select 201, 'received'::text, v_id;
end
$$;

-- Functions are executable by PUBLIC unless revoked. Only the web tier's role may call this.
revoke all on function assessment.submit_inquiry(uuid, text, text, text, text, bytea, text, text) from public;
grant execute on function assessment.submit_inquiry(uuid, text, text, text, text, bytea, text, text) to assessment_writer;

commit;
