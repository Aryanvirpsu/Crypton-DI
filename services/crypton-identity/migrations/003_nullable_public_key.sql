-- The public_key column was intended for the raw COSE-encoded key but was
-- incorrectly populated with credential_id bytes as a placeholder.
-- The `passkey` TEXT column (added in 002) is the authoritative source of
-- truth and already contains the full serialised Passkey struct (including
-- the public key). Make public_key nullable so new registrations no longer
-- write misleading data into it.
alter table credentials
    alter column public_key drop not null;
