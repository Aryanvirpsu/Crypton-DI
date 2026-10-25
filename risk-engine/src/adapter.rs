//! # Adapter Layer (reference design)
//!
//! This module defines the **interface** that connects the risk engine to the
//! Crypton identity service. It contains no Crypton-specific imports.
//!
//! The concrete adapter implementation lives in a *new file* inside
//! `crypton-gateway` or `crypton-identity` — never inside existing files.
//!
//! ## Contract
//!
//! The adapter's job is to:
//! 1. Receive an `AdapterInput` (thin wrapper around data already available
//!    in the calling handler's scope).
//! 2. Collect signals from Redis (pipelined) and PostgreSQL (single query).
//! 3. Assemble a `RiskContext`.
//! 4. Call `risk_engine::evaluate()`.
//! 5. Return the `RiskDecision` to the caller, which maps it to an HTTP
//!    response directive.
//!
//! ## Signal collection budget
//!
//! | Source              | Method         | Budget   |
//! |---------------------|----------------|----------|
//! | Redis velocity keys | MGET pipeline  | ≤ 20ms   |
//! | Redis escalation    | LRANGE         | ≤ 10ms   |
//! | PostgreSQL cred row | single SELECT  | ≤ 50ms   |
//! | GeoIP (local DB)    | in-process     | ≤ 2ms    |
//! | JWT fingerprint key | Redis GET      | ≤ 10ms   |
//! | **Total**           |                | **≤ 92ms** |
//!
//! All Redis reads are pipelined into a single round-trip.
//! If the total budget is exceeded: set `redis_signals_degraded = true`
//! (or `db_signals_degraded`) and let the engine apply the degraded-signal
//! policy (Challenge on sensitive, Allow+log on non-sensitive).

// This module is intentionally import-free. All concrete types referenced in
// the pseudocode comments are documented by name only. Import them from
// `risk_engine::*` in the concrete adapter implementation.

// ─────────────────────────────────────────────────────────────────────────────
// AdapterInput — what the calling handler provides (reference schema)
// ─────────────────────────────────────────────────────────────────────────────
//
// The concrete AdapterInput struct is defined in the host service adapter file,
// not here, because it depends on types from the host service (Uuid, IpAddr,
// RiskAction, OrgRiskLevel). Below is the reference field schema:
//
// ```rust
// pub struct AdapterInput {
//     // From JWT claims (decoded by AuthUser extractor — already in handler scope)
//     pub user_id: Uuid,
//     pub username: String,
//     pub credential_id: Option<Uuid>,
//     pub jwt_raw: Option<String>,          // raw bearer token string
//     pub jwt_issued_at: Option<i64>,       // iat claim (Unix seconds)
//     pub jwt_expires_at: Option<i64>,      // exp claim (Unix seconds)
//
//     // From request headers / middleware context
//     pub request_ip: Option<IpAddr>,
//     pub request_ua: Option<String>,
//     pub nonce: Option<String>,            // x-nonce header value
//     pub oauth_nonce: Option<String>,      // nonce from OAuth session lookup
//
//     // From routing / handler logic
//     pub action: RiskAction,
//     pub resource_id: Option<String>,
//
//     // Org-level posture (read once at startup from config or Redis)
//     pub org_risk_level: OrgRiskLevel,
// }
// ```

// ─────────────────────────────────────────────────────────────────────────────
// Pseudocode adapter function
// ─────────────────────────────────────────────────────────────────────────────
//
// The actual implementation of `evaluate_request` in the concrete adapter
// would look like the following. It is shown as commented pseudocode here
// because the concrete implementation depends on the specific Redis client
// and PostgreSQL pool in scope inside the gateway or identity service.
//
// ```rust
// pub async fn evaluate_request(
//     input: AdapterInput,
//     redis: &mut impl AsyncCommands,    // existing Redis connection
//     db: &PgPool,                       // existing PostgreSQL pool
//     geoip: &impl GeoIpResolver,        // local MaxMind wrapper
// ) -> RiskDecision {
//
//     // 1. Pipeline all Redis reads in one round-trip
//     let (
//         login_attempts_5m,
//         failed_login_attempts_1h,
//         actions_executed_5m,
//         recovery_requests_24h,
//         device_revocations_1h,
//         registrations_from_ip_10m,
//         active_session_count,
//         account_locked,
//         recovery_pending,
//         stored_fingerprint,
//         nonce_used,
//         oauth_authorize_ip,
//     ) = tokio::time::timeout(
//         Duration::from_millis(50),
//         redis_pipeline_fetch(&input, redis),
//     )
//     .await
//     .map(|r| r.unwrap_or_default())
//     .map_err(|_| /* set redis_signals_degraded = true */);
//
//     // 2. Single PostgreSQL query for credential metadata
//     let cred_row = tokio::time::timeout(
//         Duration::from_millis(100),
//         db_fetch_credential(input.credential_id, input.user_id, db),
//     )
//     .await
//     .ok().flatten();
//
//     // 3. GeoIP (local DB, no network call, < 2ms)
//     let request_geo = input.request_ip.and_then(|ip| geoip.lookup(ip));
//     let last_login_geo = cred_row.as_ref()
//         .and_then(|r| r.last_login_ip)
//         .and_then(|ip| geoip.lookup(ip));
//
//     // 4. JWT fingerprint
//     let fingerprint_key = input.jwt_raw.as_deref().map(jwt_fingerprint_key);
//     let current_fingerprint = compute_jwt_fingerprint(input.request_ip, input.request_ua.as_deref());
//
//     // 5. Assemble RiskContext
//     let ctx = RiskContext {
//         request_id: Uuid::new_v4(),
//         evaluated_at: Utc::now(),
//         user_id: input.user_id,
//         username: input.username,
//         credential_id: input.credential_id,
//         action: input.action,
//         resource_id: input.resource_id,
//
//         credential_status: cred_row.as_ref().map(|r| r.status.clone()).unwrap_or(CredentialStatus::Active),
//         credential_created_at: cred_row.as_ref().map(|r| r.created_at).unwrap_or_default(),
//         credential_last_used_at: cred_row.as_ref().and_then(|r| r.last_used_at),
//         credential_sign_count_prev: cred_row.as_ref().map(|r| r.sign_count_prev).unwrap_or(0),
//         credential_sign_count_new: cred_row.as_ref().map(|r| r.sign_count_new).unwrap_or(0),
//         credential_registered_ua: cred_row.as_ref().and_then(|r| r.user_agent.clone()),
//         credential_count_for_user: cred_row.as_ref().map(|r| r.active_cred_count).unwrap_or(1),
//
//         prior_audit_event_count: cred_row.as_ref().map(|r| r.audit_count).unwrap_or(0),
//         last_login_ip: cred_row.as_ref().and_then(|r| r.last_login_ip),
//         last_login_at: cred_row.as_ref().and_then(|r| r.last_login_at),
//         last_login_geo,
//
//         jwt_issued_at: input.jwt_issued_at.map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
//         jwt_expires_at: input.jwt_expires_at.map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
//         jwt_fingerprint_stored: stored_fingerprint,
//         jwt_fingerprint_current: Some(current_fingerprint),
//
//         request_ip: input.request_ip,
//         request_ua: input.request_ua,
//         request_geo,
//         ip_asn_type: geoip.asn_type(input.request_ip),
//
//         login_attempts_5m,
//         failed_login_attempts_1h,
//         actions_executed_5m,
//         recovery_requests_24h,
//         device_revocations_1h,
//         registrations_from_ip_10m,
//         active_session_count,
//         account_locked,
//         recovery_pending,
//         oauth_authorize_ip,
//
//         nonce_present: input.nonce.is_some(),
//         nonce_already_used: nonce_used,
//         org_risk_level: input.org_risk_level,
//         redis_signals_degraded,
//         db_signals_degraded,
//     };
//
//     // 6. Evaluate
//     let decision = risk_engine::evaluate(ctx, &store).await;
//
//     // 7. Post-decision side effects
//     //    These run after the decision is returned to the handler.
//     //    They do not block the response.
//     tokio::spawn(async move {
//         let _ = post_decision_increments(&input_copy, &decision, &store).await;
//     });
//
//     decision
// }
// ```
//
// ─────────────────────────────────────────────────────────────────────────────
// Handler integration points (where evaluate_request would be called)
// ─────────────────────────────────────────────────────────────────────────────
//
// Each call site is a NEW file in the gateway middleware stack. The existing
// handler code is never modified. The adapter intercepts at the middleware
// layer and short-circuits the response if the decision is blocking.
//
// IP-1: Login finish
//   File: crypton-gateway/src/middleware/risk_check.rs (NEW)
//   Path: POST /auth/login/finish
//   Fires: after WebAuthn assertion succeeds, before JWT is returned
//   Decision mapping:
//     Allow   → forward response as-is
//     Challenge → 428 Precondition Required + { required_action, request_id }
//     Hold    → 202 Accepted + { hold_id, reason }
//     Deny    → 403 Forbidden + { request_id }
//
// IP-2: OAuth complete
//   Path: POST /auth/oauth/complete
//   Fires: before auth code is issued
//
// IP-3: Action execute
//   Path: POST /actions/execute
//   Fires: before action is dispatched (after WebAuthn step-up)
//
// IP-4: Recovery start
//   Path: POST /recovery/start
//   Fires: before recovery record is created
//
// IP-5: Recovery approve
//   Path: POST /recovery/approve
//   Fires: before approval is recorded
//
// IP-6: Device revoke / mark-lost
//   Path: POST /devices/:id/revoke, POST /devices/:id/mark-lost
//   Fires: before status update is applied

// This file is intentionally kept as documentation + interface definition.
// No runnable Rust is emitted here to avoid coupling to specific async runtimes
// or client library versions in the host service.
