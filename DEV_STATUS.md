# CRYPTON — DEV STATUS

> Last updated: April 2026  
> Status: **ACTIVE DEVELOPMENT — DEMO READY (modular config complete)**

---

## Architecture Map

```
┌──────────────────────────────────────────────────────────────────────────┐
│  CRYPTON MONOREPO                                                        │
│                                                                          │
│  ┌─────────────────────────────────────────────────────┐                │
│  │  services/                                           │                │
│  │                                                      │                │
│  │  crypton-id      ── Rust identity backend  :8080    │                │
│  │  crypton-gateway ── Rust API gateway       :8090    │                │
│  │                                                      │                │
│  │  crypton-main    ── Marketing site         :3000    │                │
│  │  crypton-demo    ── SaaS demo app          :3001    │                │
│  │  crypton-admin   ── Operator/security panel:3002    │                │
│  └─────────────────────────────────────────────────────┘                │
│                                                                          │
│  ┌─────────────────────────────────────────────────────┐                │
│  │  infra/                                              │                │
│  │  docker-compose.yml — Postgres :5432 + Redis :6379  │                │
│  └─────────────────────────────────────────────────────┘                │
└──────────────────────────────────────────────────────────────────────────┘
```

### Request Flow

```
Browser → crypton-demo (:3001)
       → CRA proxy → crypton-gateway (:8090)
       → crypton-id (:8080)
       → Postgres + Redis
```

---

## App Roles

| App | Port | Role | What it contains |
|-----|------|------|-----------------|
| **crypton-main** | 3000 | Marketing site | Landing page, features, protocol, pricing. CTAs: Try Demo → :3001, Join Waitlist |
| **crypton-demo** | 3001 | SaaS demo app | Login (WebAuthn), Device Enrollment (Register), Dashboard, Protected Actions |
| **crypton-admin** | 3002 | Operator panel | Devices, Sessions, Audit Logs, Recovery, RBAC, Risk Intel, Policy, Org Settings |
| **crypton-id** | 8080 | Identity backend | WebAuthn auth, passkey registration, device management, recovery, JWT, sessions |
| **crypton-gateway** | 8090 | API gateway | Route proxying, CORS, auth forwarding |

### Surface Separation Rules

| Content | Lives in |
|---------|----------|
| Passkey login | `crypton-demo` |
| Device enrollment | `crypton-demo` (Register flow) |
| Protected SaaS actions | `crypton-demo` |
| Operator device list | `crypton-admin` |
| Audit logs | `crypton-admin` |
| Recovery approval | `crypton-admin` |
| Marketing copy, CTAs | `crypton-main` |
| Rust auth logic | `crypton-id` |

> [!IMPORTANT]
> Passkey creation (Register.js) MUST remain in `crypton-demo` so that WebAuthn enrollment works.  
> Do NOT move enrollment to `crypton-main`. The marketing site has no auth state.

---

## URLs

| Service | URL | Notes |
|---------|-----|-------|
| Marketing | http://localhost:3000 | "Try Demo" → :3001 |
| Demo App | http://localhost:3001 | Entry point for demos |
| Admin Panel | http://localhost:3002 | Operator access only |
| Backend | http://localhost:8080 | Internal only |
| Gateway | http://localhost:8090 | Proxied by CRA dev server |
| Health (id) | http://localhost:8080/health | Returns `{"status":"ok"}` |
| Health (gw) | http://localhost:8090/health | Returns `{"status":"ok"}` |

---

## Startup Instructions

### Prerequisites

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (for Postgres + Redis)
- [Node.js ≥ 18](https://nodejs.org/)
- [Rust + Cargo](https://rustup.rs/)

### Quick Start (local dev)

```powershell
# Start everything
.\start-all.ps1 -LocalOnly

# First-time or after code changes to Rust:
.\start-all.ps1 -LocalOnly -Rebuild -KillExisting
```

### Individual services (manual)

```powershell
# Infra only
docker compose -f infra\docker-compose.yml up -d

# Backend (crypton-id)
cd services\crypton-id
cargo run --release

# Gateway
cd services\crypton-gateway
cargo run --release

# Marketing site
cd services\crypton-main
npm start   # → :3000

# Demo app
cd services\crypton-demo
npm start   # → :3001 (set PORT=3001 first)

# Admin panel
cd services\crypton-admin
npm start   # → :3002 (set PORT=3002 first)
```

---

## Health Check Instructions

```powershell
# Check all services at once
.\status.ps1

# Manual checks
Invoke-WebRequest http://localhost:8080/health
Invoke-WebRequest http://localhost:8090/health
Invoke-WebRequest http://localhost:3000
Invoke-WebRequest http://localhost:3001
Invoke-WebRequest http://localhost:3002
```

Expected backend health response:
```json
{"status":"ok","service":"crypton-id"}
```

---

## WebAuthn Notes

> [!WARNING]
> Passkeys are bound to the exact `rpId` (relying party).  
> A passkey registered on `localhost:3001` **will not work** on a Cloudflare tunnel URL.  
> When switching modes, register a fresh passkey.

| Mode | rpId | Origin | Enrollment |
|------|------|--------|------------|
| Local dev | `localhost` | `http://localhost:3001` | Use `:3001/register` |
| Tunnel | `<tunnel>.trycloudflare.com` | `https://<tunnel>...` | Re-register on tunnel |

---

## Done vs Remaining

### ✅ Done Now

- [x] Service directory separation (`crypton-id`, `crypton-main`, `crypton-demo`, `crypton-admin`)
- [x] Demo routes cleaned — admin routes removed from `crypton-demo`
- [x] Admin routes cleaned — `demo` route removed from `crypton-admin`
- [x] Demo AppShell sidebar — shows only Demo surfaces (Dashboard, Devices, Protected Actions)
- [x] Admin AppShell sidebar — shows only Operator surfaces (no Demo Actions)
- [x] Landing page CTAs updated: "Try Demo" → `:3001`, "Join Waitlist" (placeholder)
- [x] Admin "Back to Home" → now links to `:3000` marketing site (not internal route)
- [x] Cross-links: Demo sidebar has Admin Panel link; Admin sidebar has Demo App link
- [x] `start-all.ps1` — single command boots all 5 surfaces
- [x] `status.ps1` — prints real-time health of all services + ports
- [x] `stop-all.ps1` — cleanly stops all services
- [x] `DEV_STATUS.md` — this file
- [x] Passkey/enrollment preserved inside `crypton-demo` (not moved)
- [x] SDK-first: all auth flows use `crypton.*` SDK — no raw fetch
- [x] Backend (`crypton-id`): unchanged — Rust WebAuthn backend preserved

### ✅ Resolved (this pass)

- [x] `crypton-main` Landing.js: "Join Waitlist" now opens an inline modal — captures email to localStorage, shows "You're on the list ✓" on submit. No fake API call.
- [x] `crypton-demo` Dashboard.js: "Enroll Device" button relabeled → "Add Passkey" (no behavior change)
- [x] `crypton-demo` AppShell.js Breadcrumb: verified `go("landing")` already redirects to `http://localhost:3000` ✓
- [x] `crypton-admin` App.js: `Register` import + render block removed; `Register.js` deleted; surface boundary enforced
- [x] `crypton-admin` routes.js: `/register` route removed
- [x] `crypton-admin` constants.js: `demo: "Demo Actions"` removed from PAGE_LABELS
- [x] `crypton-demo` tsconfig.json added; TypeScript SDK compiles; `sdk/client.ts` type narrowing fixed
- [x] `crypton-demo` App.js: stale `setToken` re-export removed
- [x] All three frontend surfaces build cleanly: `crypton-admin` ✓ · `crypton-main` ✓ · `crypton-demo` ✓

### ✅ Resolved (modular config pass)

- [x] All hardcoded `localhost:300x` URLs replaced with `config.js` imports across all 3 surfaces
- [x] `config.js` per surface reads from `REACT_APP_*` env vars with localhost fallbacks
- [x] `.env` + `.env.example` created for all 3 frontend services
- [x] `dev.env` + `dev.env.example` created at repo root for backend secrets
- [x] `start-all.ps1` loads backend config from `dev.env` — no hardcoded secrets in script
- [x] `crypton-demo` Dashboard — Admin Dashboard button added (links to `ADMIN_URL`)
- [x] `crypton-main` Landing.js mobile nav — Join Waitlist CTA added
- [x] Dead `crypton-admin/src/demo-app/DemoActions.js` deleted

### Still Left (production blockers only)

- [ ] Production env files for each frontend service (`.env.production`) not yet created
- [ ] CORS not scoped to exact origins in gateway — needs `ALLOWED_ORIGINS` env var
- [ ] `OAUTH_CLIENT_SECRET` is a dev default — must be secret-managed in prod
- [ ] No HTTPS in local dev — WebAuthn requires HTTPS in production (use tunnel or nginx)

---

## Remaining Risks

### Blockers for Production

| Risk | Severity | Notes |
|------|----------|-------|
| CORS not scoped to exact origins | Med | Gateway needs `ALLOWED_ORIGINS` per environment |
| `OAUTH_CLIENT_SECRET` is a dev default | HIGH | Must be secret-managed in prod |
| No HTTPS in local dev | Med | WebAuthn requires HTTPS in production — use tunnel or nginx proxy |
| JWT secret auto-generated per instance | Med | Must be pinned in prod via secret manager |

---

## Script Reference

```powershell
.\start-all.ps1 -LocalOnly              # Start all (local, localhost WebAuthn)
.\start-all.ps1 -LocalOnly -Rebuild     # Force rebuild Rust binaries
.\start-all.ps1 -UseTunnel              # Start with Cloudflare public tunnel
.\status.ps1                            # Check all service health
.\stop-all.ps1                          # Stop all services (keep Docker)
.\stop-all.ps1 -StopInfra              # Stop all + Docker
.\stop-all.ps1 -StopInfra -ClearLogs  # Nuclear clean stop
```
