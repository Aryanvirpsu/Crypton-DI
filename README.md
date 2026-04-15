# Crypton Monorepo

Zero-trust hardware device identity and authentication infrastructure.

## System Architecture

The monorepo is divided into specialized product surfaces to ensure clear ownership and separation of concerns.

### Services (`/services`)

1. **`crypton-id`** (formerly `crypton-identity`)
   - **Role**: Backend Trust Engine.
   - **Stack**: Rust (Axum), Redis, PostgreSQL.
   - **Key Logic**: Nonce generation, cryptographic signature verification, JWT issuance, session management.

2. **`crypton-main`**
   - **Role**: Public Marketing Surface.
   - **Key Logic**: Product orientation, sphere engine visualization, attack simulator, lead generation.
   - **CTAs**: Routes users to the Demo app for enrollment or the Admin panel for operators.

3. **`crypton-demo`** (formerly `demo-site`)
   - **Role**: Third-party SaaS Demo.
   - **Key Logic**: "Vault" SaaS application logic, integration with Crypton SDK for device enrollment and passkey flows.
   - **User Story**: A developer or customer uses this app to experience how Crypton feels in a real product.

4. **`crypton-admin`**
   - **Role**: Internal Operator Panel.
   - **Key Logic**: Audit logs, security policy configuration, session revocation, risk intelligence.
   - **User Story**: A security engineer or admin uses this to manage the Crypton infrastructure.

5. **`crypton-gateway`**
   - **Role**: API Gateway / Proxy.
   - **Port**: `8090` (Default).
   - **Key Logic**: Handles CORS, request forwarding, and high-level routing.

## Development Setup

### Ports (Standard Config)
- `crypton-main`: `3000`
- `crypton-demo`: `3001`
- `crypton-admin`: `3002`
- `crypton-gateway`: `8090`
- `crypton-id`: `8080`

### Quick Start (Recommended)
Use the unified scripts in the root directory:
```powershell
# Start everything (Local Auth/WebAuthn)
.\start-all.ps1 -LocalOnly

# Check system health
.\status.ps1

# Stop all services
.\stop-all.ps1
```

For more details on the architecture and manual startup, see [DEV_STATUS.md](./DEV_STATUS.md).

## Design Philosophy
Crypton follows an **SDK-First** approach. The identity infrastructure (`crypton-id`) is decoupled from the product UI. The demo app is the gold-standard implementation of the SDK.
