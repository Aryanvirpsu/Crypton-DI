-- Constrain credentials.status to only valid values.
-- Prevents rogue updates from inserting arbitrary status strings.
alter table credentials
    add constraint credentials_status_check
    check (status in ('active', 'revoked'));
