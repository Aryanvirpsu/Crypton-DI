# Crypton Console

The one frontend for the actual product: register, login with a passkey,
manage devices, recover a lost device, and (for admins) view org-wide
stats, manage user roles, and browse the audit log.

This replaces the former `crypton-demo` + `crypton-admin` split. Those two
apps were ~70% byte-for-byte duplicated, had two separate login flows and
two separate JWTs (a "user token" and an "admin token"), and several pages
that only ever rendered hardcoded mock data. This app has one login, one
token, and every page either shows real data from `crypton-id` or a real
error/empty state — no mock fallbacks.

## Running

```sh
npm install
npm start          # http://localhost:3001, proxies API calls to :8090
```

Requires `crypton-gateway` (port 8090) and `crypton-id` (port 8080) running
— see the repo root `README.md`.

## Role-gated (admin) pages

There is no separate admin login. After logging in, the app calls `GET /me`
to learn the caller's role and shows the "Operator Panel" sidebar section
only if `role` is `admin` or `super_admin`. **This is a UI convenience, not
a security boundary** — the backend's `AdminUser` extractor
(`crypton-id/src/jwt.rs`) is what actually enforces authorization on every
admin route regardless of what the sidebar shows.

On a fresh deployment with zero admins, the Admin Overview page
(`/admin`) shows a "Become the First Admin" button — this is
`POST /admin/bootstrap`, which only succeeds while no admin exists yet.

## What was intentionally not carried over from crypton-demo/crypton-admin

- **Sessions page** — no session store exists server-side; device
  list/revoke (`/devices`) is the real equivalent.
- **Risk Intelligence page** — was a "Coming Soon" placeholder with no
  backend.
- **Policy Engine / Org Settings pages** — backed a fake/removed API.
- **RBAC page** — superseded by the real `/admin/users` page here.
- **A second, separate admin login/token.**
