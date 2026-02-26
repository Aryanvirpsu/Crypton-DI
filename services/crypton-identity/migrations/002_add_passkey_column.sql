-- Checkpoint 3: store full serialised Passkey struct for authentication lookups
-- The webauthn-rs Passkey type serialises to JSON; we store it as text.
alter table credentials
    add column if not exists passkey text;
