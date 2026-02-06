Here’s a **nice, clean, professional README** you can drop straight into your repo.
It’s concise, readable, and sounds serious without being buzzword-y.

---

# Crypton

**Crypton** is a zero-trust, device-based cryptographic identity system.

Instead of passwords or OTPs, identity is defined as a **set of trusted devices**.
Each device proves control by signing fresh challenges with hardware-protected keys.
Servers store only public keys and enforce policy — nothing reusable, nothing secret.

---

## Why Crypton

Passwords are fragile:

* they’re reused
* they’re phished
* they’re leaked

Crypton replaces them with **cryptographic proof**.

* No passwords
* No shared secrets
* No SMS or email recovery
* No biometric data stored server-side

---

## Core Idea

> **Identity is a set of trusted devices.**

* Each device holds a private key (hardware-backed when available)
* The server stores only public keys
* Every action requires a fresh challenge
* Trust is per-device, not per-account
* Devices can be revoked at any time

---

## Architecture Overview

```
User
  └── Device (hardware-backed key)
        └── Signs challenge
              └── Crypton Identity Service
                    ├── Verify signature
                    ├── Enforce policy
                    └── Issue access
```

---

## Zero-Trust Principles

* No passwords or shared secrets stored
* Every request uses a fresh challenge
* Replay protection via nonces
* Devices are revocable
* Step-up verification for sensitive actions
* Deny-by-default access policies
* Full audit logging

---

## Phases

### Phase A — Passkeys (WebAuthn)

* Passwordless registration and login
* Device registry
* Device revoke
* Protected API demo
* Passkeys act as recovery anchor

### Phase B — Native Device Keys

* Hardware-backed Ed25519 device keys
* Device approval via existing trusted device
* Key rotation and revocation
* Passkeys retained only for recovery

---

## Recovery Model

Crypton uses **zero-trust recovery**.

* Recovery requires a valid passkey proof
* A time-locked recovery event is created (e.g. 24 hours)
* Any existing trusted device can cancel during the window
* After the lock expires:

  * new device is enrolled
  * old devices are revoked

No SMS. No email. No humans in the loop.

---

## Tech Stack

* **Backend:** Rust + Axum
* **Database:** PostgreSQL
* **Cache / Nonces / Rate limits:** Redis
* **Admin & Demo UI:** Next.js
* **Local Dev:** Docker Compose

---

## Repository Structure

```
crypton/
├─ services/
│  ├─ crypton-identity/    # Identity service (Axum)
│  ├─ crypton-gateway/     # Policy & enforcement (future)
│  └─ crypton-admin/       # Admin & demo UI (future)
├─ libs/
│  └─ contracts/           # Shared API contracts
├─ infra/
│  └─ docker-compose.yml
└─ docs/
   ├─ SPEC.md
   ├─ API_CONTRACTS.md
   └─ GIT_WORKFLOW.md
```

---

## Local Development

### Requirements

* Rust (stable)
* Docker + Docker Compose

### Start dependencies

```bash
docker compose -f infra/docker-compose.yml up -d
```

### Run identity service

```bash
cd services/crypton-identity
cargo run
```

### Health check

```bash
curl http://localhost:8080/health
```

Expected response:

```
ok
```

---

## What This Project Is (and Isn’t)

**Included**

* Device-based identity
* Passkeys (WebAuthn)
* Cryptographic proof verification
* Zero-trust recovery
* Audit logging

**Not included (yet)**

* Messaging protocols
* Payments or wallets
* KYC / compliance
* Blockchain / DID networks
* Zero-knowledge proofs

---

## Status

🚧 **Active development**
Phase A (Passkeys) in progress.

---

## Vision

Crypton treats identity as infrastructure — not an account, not a password, not a profile.

Just **devices, keys, and cryptographic proof**.

---

