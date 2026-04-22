# Graph Report - .  (2026-04-21)

## Corpus Check
- 155 files · ~82,403 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 604 nodes · 930 edges · 73 communities detected
- Extraction: 65% EXTRACTED · 35% INFERRED · 0% AMBIGUOUS · INFERRED: 325 edges (avg confidence: 0.81)
- Token cost: 3,072 input · 1,864 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Action Execution & Audit Logging|Action Execution & Audit Logging]]
- [[_COMMUNITY_Action RequestResponse Types|Action Request/Response Types]]
- [[_COMMUNITY_Infrastructure & Service Config|Infrastructure & Service Config]]
- [[_COMMUNITY_SDK Actions Module|SDK Actions Module]]
- [[_COMMUNITY_Admin Routes & Device Management|Admin Routes & Device Management]]
- [[_COMMUNITY_JWT Auth & Claims Validation|JWT Auth & Claims Validation]]
- [[_COMMUNITY_Marketing Landing Page Components|Marketing Landing Page Components]]
- [[_COMMUNITY_React Logo Visual Elements|React Logo Visual Elements]]
- [[_COMMUNITY_User Dashboard UI|User Dashboard UI]]
- [[_COMMUNITY_Auth Flow & Registration|Auth Flow & Registration]]
- [[_COMMUNITY_OAuth Authorization Handler|OAuth Authorization Handler]]
- [[_COMMUNITY_Build & CI Scripts|Build & CI Scripts]]
- [[_COMMUNITY_Crypton Brand & Iconography|Crypton Brand & Iconography]]
- [[_COMMUNITY_Recovery Module|Recovery Module]]
- [[_COMMUNITY_App Shell & Navigation|App Shell & Navigation]]
- [[_COMMUNITY_Create React App Scaffolding|Create React App Scaffolding]]
- [[_COMMUNITY_React Default Assets|React Default Assets]]
- [[_COMMUNITY_Admin Dashboard UI|Admin Dashboard UI]]
- [[_COMMUNITY_API Response Types|API Response Types]]
- [[_COMMUNITY_Crypton Admin Visual Identity|Crypton Admin Visual Identity]]
- [[_COMMUNITY_SDK Devices Module|SDK Devices Module]]
- [[_COMMUNITY_Device Management UI|Device Management UI]]
- [[_COMMUNITY_Admin Network Visualizer|Admin Network Visualizer]]
- [[_COMMUNITY_Shared Button Components|Shared Button Components]]
- [[_COMMUNITY_Dev Status Scripts|Dev Status Scripts]]
- [[_COMMUNITY_SDK Error Types|SDK Error Types]]
- [[_COMMUNITY_Attack Simulator|Attack Simulator]]
- [[_COMMUNITY_RBAC  Role Management|RBAC / Role Management]]
- [[_COMMUNITY_CryptonClient Entry Point|CryptonClient Entry Point]]
- [[_COMMUNITY_WebAuthn Test Mocks|WebAuthn Test Mocks]]
- [[_COMMUNITY_Audit Logs Page|Audit Logs Page]]
- [[_COMMUNITY_Org Settings Page|Org Settings Page]]
- [[_COMMUNITY_Policy Engine Page|Policy Engine Page]]
- [[_COMMUNITY_Recovery Page|Recovery Page]]
- [[_COMMUNITY_Web Vitals Reporter|Web Vitals Reporter]]
- [[_COMMUNITY_Risk Intel Page|Risk Intel Page]]
- [[_COMMUNITY_Client-Side Routing|Client-Side Routing]]
- [[_COMMUNITY_SDK Test Suite|SDK Test Suite]]
- [[_COMMUNITY_Sessions Page|Sessions Page]]
- [[_COMMUNITY_Demo Dev Server|Demo Dev Server]]
- [[_COMMUNITY_Auth Regression Tests|Auth Regression Tests]]
- [[_COMMUNITY_Animated Heading Component|Animated Heading Component]]
- [[_COMMUNITY_Panel Wall Animation|Panel Wall Animation]]
- [[_COMMUNITY_SDK Singleton Removal Rationale|SDK Singleton Removal Rationale]]
- [[_COMMUNITY_SDK Consolidation Rationale|SDK Consolidation Rationale]]
- [[_COMMUNITY_Jest Config|Jest Config]]
- [[_COMMUNITY_SDK Type Definitions|SDK Type Definitions]]
- [[_COMMUNITY_SDK Integration Tests|SDK Integration Tests]]
- [[_COMMUNITY_Jest Setup|Jest Setup]]
- [[_COMMUNITY_Demo App Test|Demo App Test]]
- [[_COMMUNITY_Demo Config|Demo Config]]
- [[_COMMUNITY_Demo Constants|Demo Constants]]
- [[_COMMUNITY_Demo Entry Point|Demo Entry Point]]
- [[_COMMUNITY_Demo SDK Wrapper|Demo SDK Wrapper]]
- [[_COMMUNITY_Demo Test Setup|Demo Test Setup]]
- [[_COMMUNITY_Admin App Test|Admin App Test]]
- [[_COMMUNITY_Admin Config|Admin Config]]
- [[_COMMUNITY_Admin Constants|Admin Constants]]
- [[_COMMUNITY_Admin Entry Point|Admin Entry Point]]
- [[_COMMUNITY_Admin SDK Wrapper|Admin SDK Wrapper]]
- [[_COMMUNITY_Admin Test Setup|Admin Test Setup]]
- [[_COMMUNITY_Rust Audit Module|Rust Audit Module]]
- [[_COMMUNITY_Rust Middleware Module|Rust Middleware Module]]
- [[_COMMUNITY_Rust Routes Module|Rust Routes Module]]
- [[_COMMUNITY_Rust DB Layer|Rust DB Layer]]
- [[_COMMUNITY_Rust DB Module|Rust DB Module]]
- [[_COMMUNITY_Rust DB Models|Rust DB Models]]
- [[_COMMUNITY_Rust Redis Layer|Rust Redis Layer]]
- [[_COMMUNITY_Rust Redis Module|Rust Redis Module]]
- [[_COMMUNITY_Main Auth Module|Main Auth Module]]
- [[_COMMUNITY_Main Config|Main Config]]
- [[_COMMUNITY_Main Constants|Main Constants]]
- [[_COMMUNITY_Main Entry Point|Main Entry Point]]

## God Nodes (most connected - your core abstractions)
1. `OK()` - 65 edges
2. `log_event()` - 15 edges
3. `React Logo (Atomic Orbital Design)` - 14 edges
4. `@crypton/sdk (Production SDK Package)` - 13 edges
5. `register_finish()` - 11 edges
6. `row_to_recovery()` - 11 edges
7. `public_complete_recovery()` - 11 edges
8. `RecoveryModule` - 10 edges
9. `login_finish()` - 10 edges
10. `logout()` - 10 edges

## Surprising Connections (you probably didn't know these)
- `WebAuthn / Passkey Authentication` --semantically_similar_to--> `Browser-Only Requirement (navigator.credentials)`  [INFERRED] [semantically similar]
  DEV_STATUS.md → libs/crypton-sdk/README.md
- `get_i32()` --calls--> `OK()`  [INFERRED]
  services\crypton-gateway\src\routes\diagnostics.rs → stop-all.ps1
- `ttl()` --calls--> `OK()`  [INFERRED]
  services\crypton-gateway\src\routes\diagnostics.rs → stop-all.ps1
- `user()` --calls--> `OK()`  [INFERRED]
  services\crypton-gateway\src\routes\diagnostics.rs → stop-all.ps1
- `device()` --calls--> `OK()`  [INFERRED]
  services\crypton-gateway\src\routes\diagnostics.rs → stop-all.ps1

## Hyperedges (group relationships)
- **Browser → Demo → Gateway → Identity → DB Request Flow** — devstatus_crypton_demo, devstatus_crypton_gateway, devstatus_crypton_id [EXTRACTED 1.00]
- **SDK-First: crypton-admin and crypton-demo consume @crypton/sdk as first-party integrators** — sdkplan_crypton_sdk, devstatus_crypton_admin, devstatus_crypton_demo [EXTRACTED 1.00]
- **Gateway Security Layer: Rate Limiting + Replay Protection + Policy Engine** — gwreadme_rate_limiting, gwreadme_replay_protection, gwreadme_policy_engine [EXTRACTED 0.95]

## Communities

### Community 0 - "Action Execution & Audit Logging"
Cohesion: 0.06
Nodes (77): action_challenge(), action_execute(), log_event(), list_audit_logs(), authenticated_user_id_from_headers(), consume_recovery_enrollment_grant(), get_recovery_enrollment_grant(), login_finish() (+69 more)

### Community 1 - "Action Request/Response Types"
Cohesion: 0.04
Nodes (35): ActionChallengeReq, ActionChallengeResp, ActionExecuteReq, ActionExecuteResp, router(), App(), canRenderLocalAuthUi(), FontLink() (+27 more)

### Community 2 - "Infrastructure & Service Config"
Cohesion: 0.05
Nodes (51): crypton-admin (Create React App), Cloudflare Tunnel (Public Exposure), config.js (Modular Environment Config), CORS Not Scoped to Exact Origins (Risk), crypton-admin (Operator Panel), crypton-demo (SaaS Demo App), crypton-gateway (API Gateway), crypton-id (Identity Backend) (+43 more)

### Community 3 - "SDK Actions Module"
Cohesion: 0.05
Nodes (22): ActionsModule, wrapWebAuthnError(), _authFetch(), _adminFetch(), _adminWrap(), _request(), _wrap(), AuthModule (+14 more)

### Community 4 - "Admin Routes & Device Management"
Cohesion: 0.07
Nodes (25): ActivityItem, delete_all_sessions(), delete_passkey(), delete_session(), get_org(), list_passkeys(), list_policies(), list_sessions() (+17 more)

### Community 5 - "JWT Auth & Claims Validation"
Cohesion: 0.08
Nodes (21): Claims, validate_jwt(), verify_opaque_token(), device(), exists(), get_i32(), ip(), ttl() (+13 more)

### Community 6 - "Marketing Landing Page Components"
Cohesion: 0.19
Nodes (17): ArchDiagram(), AttackSimTrigger(), AttackTerminal(), BeforeAfter(), clampT(), CodeBlock(), easeInOutCubic(), easeOutCubic() (+9 more)

### Community 7 - "React Logo Visual Elements"
Cohesion: 0.19
Nodes (19): Atomic Orbital Visual Motif, Atomic Orbital / Electron Orbit Symbolism, Central Circle / Nucleus, React Brand Cyan Color (#61DAFB), Create React App Boilerplate, Crypton Admin Service, Crypton Demo Application, Crypton Frontend Application (+11 more)

### Community 8 - "User Dashboard UI"
Cohesion: 0.14
Nodes (4): Dashboard(), useReveal(), useToasts(), UserDashboard()

### Community 9 - "Auth Flow & Registration"
Cohesion: 0.18
Nodes (9): getValidToken(), Register(), parseJwt(), INFO(), Kill-Port(), Stop-Tree(), WARN(), doLogin() (+1 more)

### Community 10 - "OAuth Authorization Handler"
Cohesion: 0.2
Nodes (12): authorize(), AuthorizeParams, cfg_client_id(), cfg_client_secret(), cfg_redirect_uris(), CompleteBody, CompleteResponse, oauth_complete() (+4 more)

### Community 11 - "Build & CI Scripts"
Cohesion: 0.35
Nodes (11): Banner(), Build-Rust(), FAIL(), INFO(), Kill-Port(), Log(), OK(), Start-Frontend() (+3 more)

### Community 12 - "Crypton Brand & Iconography"
Cohesion: 0.27
Nodes (13): Blockchain / Chain Symbolism, Blockchain Segmentation / Digital Blocks Motif, Crypton Brand Identity, C Letterform / Partial Ring Shape, Cryptocurrency / Crypto Brand Identity, Cryptography / Security Concept, Cycle / Rotation / Loading Motif, Near-Black Color (#0A0A0A) — Brand Dark Background (+5 more)

### Community 13 - "Recovery Module"
Cohesion: 0.18
Nodes (1): RecoveryModule

### Community 14 - "App Shell & Navigation"
Cohesion: 0.31
Nodes (5): AppShell(), Breadcrumb(), MobileHeader(), Sidebar(), useAuth()

### Community 15 - "Create React App Scaffolding"
Cohesion: 0.39
Nodes (9): Atom / Electron Orbit Symbol, Atomic / Orbital Symbol, Create React App (CRA) Scaffold, Crypton Admin Service, Crypton Demo Web Application, Light Blue (#61DAFB) Brand Color, Progressive Web App Icon (192px), React JavaScript Framework (+1 more)

### Community 16 - "React Default Assets"
Cohesion: 0.39
Nodes (9): Atomic Orbit Visual Motif, Atomic Orbital Visual Design, React Brand Color #61DAFB (Light Blue), Create React App Boilerplate, Crypton Admin Service, Crypton Demo Service (Frontend), Cyan / Sky-Blue Brand Color, React JavaScript Framework (+1 more)

### Community 17 - "Admin Dashboard UI"
Cohesion: 0.29
Nodes (0): 

### Community 18 - "API Response Types"
Cohesion: 0.29
Nodes (5): ApiErrorBody, ApiResponse, AppJson, AppJson<T>, ErrorEnvelope

### Community 19 - "Crypton Admin Visual Identity"
Cohesion: 0.43
Nodes (7): Admin Panel Application, Coin or Token Visual Motif, Crypton Brand Identity, Digital Segmented Aesthetic, Crypton Admin Favicon, Dark Circular Badge Background, Dashed Arc C-Letterform Symbol

### Community 20 - "SDK Devices Module"
Cohesion: 0.33
Nodes (1): DevicesModule

### Community 21 - "Device Management UI"
Cohesion: 0.53
Nodes (4): DeviceCard(), Devices(), formatRelativeTime(), parseUserAgent()

### Community 22 - "Admin Network Visualizer"
Cohesion: 0.6
Nodes (3): Admin(), AdminCard(), NetworkCanvas()

### Community 23 - "Shared Button Components"
Cohesion: 0.6
Nodes (2): BtnF(), BtnO()

### Community 24 - "Dev Status Scripts"
Cohesion: 0.5
Nodes (0): 

### Community 25 - "SDK Error Types"
Cohesion: 0.5
Nodes (1): CryptonError

### Community 26 - "Attack Simulator"
Cohesion: 0.5
Nodes (1): AttackSim()

### Community 27 - "RBAC / Role Management"
Cohesion: 0.67
Nodes (2): normalizeRole(), RBAC()

### Community 28 - "CryptonClient Entry Point"
Cohesion: 0.67
Nodes (1): CryptonClient

### Community 29 - "WebAuthn Test Mocks"
Cohesion: 0.67
Nodes (0): 

### Community 30 - "Audit Logs Page"
Cohesion: 0.67
Nodes (1): AuditLogs()

### Community 31 - "Org Settings Page"
Cohesion: 0.67
Nodes (1): OrgSettings()

### Community 32 - "Policy Engine Page"
Cohesion: 0.67
Nodes (1): PolicyEngine()

### Community 33 - "Recovery Page"
Cohesion: 0.67
Nodes (1): Recovery()

### Community 34 - "Web Vitals Reporter"
Cohesion: 0.67
Nodes (1): reportWebVitals()

### Community 35 - "Risk Intel Page"
Cohesion: 0.67
Nodes (1): RiskIntel()

### Community 36 - "Client-Side Routing"
Cohesion: 0.67
Nodes (1): getPageFromPath()

### Community 37 - "SDK Test Suite"
Cohesion: 0.67
Nodes (1): mockResponse()

### Community 38 - "Sessions Page"
Cohesion: 0.67
Nodes (1): Sessions()

### Community 39 - "Demo Dev Server"
Cohesion: 0.67
Nodes (0): 

### Community 40 - "Auth Regression Tests"
Cohesion: 1.0
Nodes (0): 

### Community 41 - "Animated Heading Component"
Cohesion: 1.0
Nodes (0): 

### Community 42 - "Panel Wall Animation"
Cohesion: 1.0
Nodes (0): 

### Community 43 - "SDK Singleton Removal Rationale"
Cohesion: 1.0
Nodes (2): BLOCKER-2: Singleton Baked to CRA Env Variable, Rationale: Remove Pre-Configured SDK Singleton Export

### Community 44 - "SDK Consolidation Rationale"
Cohesion: 1.0
Nodes (2): BLOCKER-1: Three SDK Copies, No Package Boundary, Rationale: Single Canonical SDK Source in libs/crypton-sdk

### Community 45 - "Jest Config"
Cohesion: 1.0
Nodes (0): 

### Community 46 - "SDK Type Definitions"
Cohesion: 1.0
Nodes (0): 

### Community 47 - "SDK Integration Tests"
Cohesion: 1.0
Nodes (0): 

### Community 48 - "Jest Setup"
Cohesion: 1.0
Nodes (0): 

### Community 49 - "Demo App Test"
Cohesion: 1.0
Nodes (0): 

### Community 50 - "Demo Config"
Cohesion: 1.0
Nodes (0): 

### Community 51 - "Demo Constants"
Cohesion: 1.0
Nodes (0): 

### Community 52 - "Demo Entry Point"
Cohesion: 1.0
Nodes (0): 

### Community 53 - "Demo SDK Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 54 - "Demo Test Setup"
Cohesion: 1.0
Nodes (0): 

### Community 55 - "Admin App Test"
Cohesion: 1.0
Nodes (0): 

### Community 56 - "Admin Config"
Cohesion: 1.0
Nodes (0): 

### Community 57 - "Admin Constants"
Cohesion: 1.0
Nodes (0): 

### Community 58 - "Admin Entry Point"
Cohesion: 1.0
Nodes (0): 

### Community 59 - "Admin SDK Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 60 - "Admin Test Setup"
Cohesion: 1.0
Nodes (0): 

### Community 61 - "Rust Audit Module"
Cohesion: 1.0
Nodes (0): 

### Community 62 - "Rust Middleware Module"
Cohesion: 1.0
Nodes (0): 

### Community 63 - "Rust Routes Module"
Cohesion: 1.0
Nodes (0): 

### Community 64 - "Rust DB Layer"
Cohesion: 1.0
Nodes (0): 

### Community 65 - "Rust DB Module"
Cohesion: 1.0
Nodes (0): 

### Community 66 - "Rust DB Models"
Cohesion: 1.0
Nodes (0): 

### Community 67 - "Rust Redis Layer"
Cohesion: 1.0
Nodes (0): 

### Community 68 - "Rust Redis Module"
Cohesion: 1.0
Nodes (0): 

### Community 69 - "Main Auth Module"
Cohesion: 1.0
Nodes (0): 

### Community 70 - "Main Config"
Cohesion: 1.0
Nodes (0): 

### Community 71 - "Main Constants"
Cohesion: 1.0
Nodes (0): 

### Community 72 - "Main Entry Point"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **88 isolated node(s):** `GatewayState`, `AuditEvent`, `Claims`, `PolicyContext`, `AppJson<T>` (+83 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Auth Regression Tests`** (2 nodes): `makeJwt()`, `auth.regression-1.test.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Animated Heading Component`** (2 nodes): `AnimatedHeading()`, `AnimatedHeading.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Panel Wall Animation`** (2 nodes): `PanelWall()`, `PanelWall.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `SDK Singleton Removal Rationale`** (2 nodes): `BLOCKER-2: Singleton Baked to CRA Env Variable`, `Rationale: Remove Pre-Configured SDK Singleton Export`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `SDK Consolidation Rationale`** (2 nodes): `BLOCKER-1: Three SDK Copies, No Package Boundary`, `Rationale: Single Canonical SDK Source in libs/crypton-sdk`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Jest Config`** (1 nodes): `jest.config.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `SDK Type Definitions`** (1 nodes): `types.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `SDK Integration Tests`** (1 nodes): `integration.test.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Jest Setup`** (1 nodes): `jest.setup.ts`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo App Test`** (1 nodes): `App.test.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo Config`** (1 nodes): `config.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo Constants`** (1 nodes): `constants.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo Entry Point`** (1 nodes): `index.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo SDK Wrapper`** (1 nodes): `sdk.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Demo Test Setup`** (1 nodes): `setupTests.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin App Test`** (1 nodes): `App.test.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin Config`** (1 nodes): `config.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin Constants`** (1 nodes): `constants.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin Entry Point`** (1 nodes): `index.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin SDK Wrapper`** (1 nodes): `sdk.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Admin Test Setup`** (1 nodes): `setupTests.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust Audit Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust Middleware Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust Routes Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust DB Layer`** (1 nodes): `db.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust DB Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust DB Models`** (1 nodes): `models.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust Redis Layer`** (1 nodes): `redis.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Rust Redis Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Main Auth Module`** (1 nodes): `auth.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Main Config`** (1 nodes): `config.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Main Constants`** (1 nodes): `constants.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Main Entry Point`** (1 nodes): `index.js`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OK()` connect `Action Execution & Audit Logging` to `Action Request/Response Types`, `JWT Auth & Claims Validation`, `Auth Flow & Registration`, `OAuth Authorization Handler`, `API Response Types`?**
  _High betweenness centrality (0.150) - this node is a cross-community bridge._
- **Why does `useSphereIntro()` connect `Marketing Landing Page Components` to `Action Execution & Audit Logging`?**
  _High betweenness centrality (0.041) - this node is a cross-community bridge._
- **Why does `App()` connect `Action Request/Response Types` to `User Dashboard UI`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
- **Are the 63 inferred relationships involving `OK()` (e.g. with `main()` and `.new()`) actually correct?**
  _`OK()` has 63 INFERRED edges - model-reasoned connections that need verification._
- **Are the 14 inferred relationships involving `log_event()` (e.g. with `.execute()` and `action_execute()`) actually correct?**
  _`log_event()` has 14 INFERRED edges - model-reasoned connections that need verification._
- **Are the 4 inferred relationships involving `React Logo (Atomic Orbital Design)` (e.g. with `Crypton Admin Service` and `Create React App Boilerplate`) actually correct?**
  _`React Logo (Atomic Orbital Design)` has 4 INFERRED edges - model-reasoned connections that need verification._
- **Are the 8 inferred relationships involving `register_finish()` (e.g. with `OK()` and `challenge_key()`) actually correct?**
  _`register_finish()` has 8 INFERRED edges - model-reasoned connections that need verification._