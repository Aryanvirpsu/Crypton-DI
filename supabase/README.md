# Supabase: assessment lead storage

One migration, applied by hand to a dedicated project (intended name: `crypton-production`).
Apply it once per environment; the checklist below covers the steps.

```
supabase/migrations/20261015000000_assessment_inquiries.sql
```

The design, the audit of the legacy service and the threat model are in
[`services/crypton-main/api/README.md`](../services/crypton-main/api/README.md).

## What the migration creates

Private schema `assessment` (not exposed by Supabase's REST API), two RLS-enabled tables, the
`submit_inquiry` and `prune_expired` functions, and two `NOLOGIN` roles: `assessment_writer` (web tier,
EXECUTE on one function only) and `assessment_operator` (SELECT, UPDATE of `status` only). It grants nothing to
`anon`, `authenticated`, `service_role` or `PUBLIC`.

## Deployment checklist (each step needs the owner's approval)

1. **Billing check.** Confirm the organisation's plan, how many projects it already has and whether another
   project adds a charge. (The Supabase connector was not connected when this was written, so none of this has
   been inspected.)
2. **Create the project** `crypton-production`, in the region nearest the Vercel Functions region.
3. **Apply the migration** (SQL editor or `supabase db push`) as the project's `postgres` role.
4. **Set role passwords** out of band, with generated passwords, in the SQL editor:
   ```sql
   alter role assessment_writer   login password '<generated>';
   alter role assessment_operator login password '<generated>';
   ```
5. **Connection strings.** Writer: Supavisor pooler, *transaction mode* (port 6543). Supavisor expects the
   project reference in the username (`assessment_writer.<project-ref>`); this form works for custom
   roles; confirm it with a test login. Operator: direct connection from the operator's machine.
6. **TLS.** Download the project's CA certificate from the dashboard and set `ASSESSMENT_DATABASE_CA`, keeping
   `ASSESSMENT_DATABASE_SSL=verify`.
7. **Verify the lock-down on the live project** (anon key, from outside):
   `GET https://<ref>.supabase.co/rest/v1/assessment_inquiries` must not succeed, and the `assessment` schema must
   not be listed under *API settings -> Exposed schemas*.
8. **Vercel environment variables** (Production and Preview, secrets as Sensitive): `ASSESSMENT_MASTER_KEY`,
   `ASSESSMENT_DATABASE_URL`, `ASSESSMENT_ALLOWED_ORIGINS`, `ASSESSMENT_DATABASE_CA`.
9. **Vercel Firewall:** add a rate-limit rule on `POST /api/assessment`.
10. **Preview deployment first**, never straight to production. Confirm the `postgres` driver bundles correctly
    in the workspace build (confirm this on the Preview build), submit a lead, read it back with `npm run inbox`.
11. Optional: `pg_cron` job calling `assessment.prune_expired()` daily (the function also prunes opportunistically).

## Rotating the master key

Set the new key as `ASSESSMENT_MASTER_KEY` in Vercel and keep the old one as `ASSESSMENT_MASTER_KEY_PREVIOUS` on
the operator machine. New leads carry the new `key_id`; old leads stay readable. Challenge tokens signed with the
old key stop verifying, which only affects forms open at that moment.
