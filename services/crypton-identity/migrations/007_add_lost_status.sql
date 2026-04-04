-- Allow 'lost' as a valid credential status in addition to 'active' and 'revoked'.
-- Must drop and recreate the CHECK constraint since ALTER TABLE ... ADD CONSTRAINT
-- on an existing constraint is not supported in PostgreSQL.
alter table credentials drop constraint credentials_status_check;

alter table credentials
    add constraint credentials_status_check
    check (status in ('active', 'lost', 'revoked'));
